//! Incoming-webhook receiver + the shared "JSON payload → profile" routine.
//!
//! A desktop app on `localhost` isn't reachable from the public internet, so the
//! live receiver is meant to be fronted by a tunnel (ngrok / Cloudflare Tunnel)
//! or a LAN automation relay (Make / n8n / Zapier). The exact same mapping powers
//! the always-works "New profile from pasted JSON" box in the UI, so a profile is
//! built identically whether the data arrives over HTTP or is pasted by hand.
//!
//! Security posture: bound to `127.0.0.1` only, and every request must carry the
//! shared `?token=` secret. The server only runs while the app is open and the
//! receiver is enabled in Settings.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

use crate::profiles::{self, Profile, ProfilesState};
use crate::settings::WebhookConfig;

// ─── Shared mapping: JSON → Profile ──────────────────────────────────────────

/// Render a JSON scalar as a plain string; objects/arrays are compacted to JSON.
fn val_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// Substitute `{{key}}` placeholders in `template` from the incoming object's
/// top-level keys.
fn render_name(template: &str, obj: &serde_json::Map<String, Value>) -> String {
    let mut out = template.to_string();
    for (k, v) in obj {
        out = out.replace(&format!("{{{{{k}}}}}"), &val_to_string(v));
    }
    // Drop any placeholders that had no matching key.
    while let Some(start) = out.find("{{") {
        if let Some(end) = out[start..].find("}}") {
            out.replace_range(start..start + end + 2, "");
        } else {
            break;
        }
    }
    out.trim().to_string()
}

/// Turn an incoming JSON payload into a `Profile` using the configured field
/// mapping. Returns `None` when the payload isn't a JSON object.
pub fn build_profile_from_json(cfg: &WebhookConfig, payload: &Value) -> Option<Profile> {
    let obj = payload.as_object()?;

    let mut values = std::collections::BTreeMap::new();

    // Explicit mappings first (incoming key `from` → variable `to`).
    let mapped_keys: std::collections::HashSet<&str> =
        cfg.mappings.iter().map(|m| m.from.as_str()).collect();
    for m in &cfg.mappings {
        if let Some(v) = obj.get(&m.from) {
            let to = m.to.trim();
            if !to.is_empty() {
                values.insert(to.to_string(), val_to_string(v));
            }
        }
    }

    // Passthrough: any unmapped key becomes a variable of the same name.
    if cfg.passthrough {
        for (k, v) in obj {
            if !mapped_keys.contains(k.as_str()) {
                values.entry(k.clone()).or_insert_with(|| val_to_string(v));
            }
        }
    }

    let mut name = render_name(&cfg.name_template, obj);
    if name.is_empty() {
        name = values.values().next().cloned().unwrap_or_else(|| "Webhook profile".into());
    }

    Some(Profile { id: String::new(), name, values, source: "webhook".into() })
}

/// Upsert a freshly-built profile into the store, persist, and notify the UI.
fn ingest_profile(app: &AppHandle, profile: Profile) -> String {
    let name = profile.name.clone();
    let state = app.state::<ProfilesState>();
    {
        let mut data = state.data.lock().unwrap();
        profiles::upsert_profile(&mut data, profile);
    }
    state.save();
    let _ = app.emit("profiles-changed", ());
    name
}

// ─── Live receiver (tiny_http on a background thread) ────────────────────────

struct RunningServer {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    port: u16,
}

/// Tauri-managed handle to the (optionally running) receiver thread.
#[derive(Default)]
pub struct WebhookController {
    inner: Mutex<Option<RunningServer>>,
}

impl WebhookController {
    /// Stop any running server and, if `cfg.enabled`, start a fresh one.
    pub fn apply(&self, app: &AppHandle, cfg: &WebhookConfig) {
        self.stop();
        if cfg.enabled {
            self.start(app, cfg);
        }
    }

    fn stop(&self) {
        if let Some(mut running) = self.inner.lock().unwrap().take() {
            running.stop.store(true, Ordering::Relaxed);
            if let Some(h) = running.handle.take() {
                let _ = h.join();
            }
        }
    }

    fn start(&self, app: &AppHandle, cfg: &WebhookConfig) {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let app = app.clone();
        let token = cfg.token.clone();
        let port = cfg.port;

        let handle = std::thread::spawn(move || {
            let server = match tiny_http::Server::http(("127.0.0.1", port)) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[castline] webhook receiver failed to bind 127.0.0.1:{port}: {e}");
                    return;
                }
            };
            eprintln!("[castline] webhook receiver listening on http://127.0.0.1:{port}/hook");

            while !stop_thread.load(Ordering::Relaxed) {
                match server.recv_timeout(Duration::from_millis(500)) {
                    Ok(Some(request)) => handle_request(&app, request, &token),
                    Ok(None) => {} // timeout — re-check the stop flag
                    Err(_) => break,
                }
            }
        });

        *self.inner.lock().unwrap() = Some(RunningServer { stop, handle: Some(handle), port });
    }

    /// The port the receiver is currently bound to (if running).
    pub fn active_port(&self) -> Option<u16> {
        self.inner.lock().unwrap().as_ref().map(|r| r.port)
    }
}

fn query_token(url: &str) -> Option<String> {
    let q = url.split_once('?')?.1;
    for pair in q.split('&') {
        if let Some(v) = pair.strip_prefix("token=") {
            return Some(urldecode(v));
        }
    }
    None
}

/// Minimal percent-decoder (enough for tokens: handles %XX and '+').
fn urldecode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    out.push((hi * 16 + lo) as u8);
                    i += 2;
                } else {
                    out.push(b'%');
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn respond(request: tiny_http::Request, status: u16, body: &str) {
    let header = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .expect("valid header");
    let response = tiny_http::Response::from_string(body)
        .with_status_code(status)
        .with_header(header);
    let _ = request.respond(response);
}

fn handle_request(app: &AppHandle, mut request: tiny_http::Request, token: &str) {
    // Method must be POST.
    if request.method() != &tiny_http::Method::Post {
        respond(request, 405, r#"{"ok":false,"error":"use POST"}"#);
        return;
    }
    // Token must match.
    if token.is_empty() || query_token(request.url()).as_deref() != Some(token) {
        respond(request, 401, r#"{"ok":false,"error":"invalid or missing token"}"#);
        return;
    }
    // Read + parse the JSON body.
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        respond(request, 400, r#"{"ok":false,"error":"could not read body"}"#);
        return;
    }
    let payload: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => {
            respond(request, 400, r#"{"ok":false,"error":"body is not valid JSON"}"#);
            return;
        }
    };

    let cfg = app.state::<crate::settings::SettingsState>().snapshot().webhook;
    match build_profile_from_json(&cfg, &payload) {
        Some(profile) => {
            let name = ingest_profile(app, profile);
            let escaped = name.replace('\\', "\\\\").replace('"', "\\\"");
            respond(request, 200, &format!(r#"{{"ok":true,"profile":"{escaped}"}}"#));
        }
        None => respond(request, 400, r#"{"ok":false,"error":"payload must be a JSON object"}"#),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{FieldMap, WebhookConfig};

    fn cfg_with(mappings: Vec<(&str, &str)>, passthrough: bool, name_tpl: &str) -> WebhookConfig {
        WebhookConfig {
            enabled: true,
            port: 8787,
            token: "secret".into(),
            name_template: name_tpl.into(),
            mappings: mappings
                .into_iter()
                .map(|(f, t)| FieldMap { from: f.into(), to: t.into() })
                .collect(),
            passthrough,
        }
    }

    #[test]
    fn maps_fields_and_builds_name() {
        let cfg = cfg_with(
            vec![("first_name", "firstName"), ("last_name", "lastName")],
            false,
            "{{first_name}} {{last_name}}",
        );
        let payload = serde_json::json!({
            "first_name": "Sam", "last_name": "Rivera", "email": "sam@x.com"
        });
        let p = build_profile_from_json(&cfg, &payload).unwrap();
        assert_eq!(p.name, "Sam Rivera");
        assert_eq!(p.values.get("firstName").unwrap(), "Sam");
        assert_eq!(p.values.get("lastName").unwrap(), "Rivera");
        // email was not mapped and passthrough is off → absent.
        assert!(!p.values.contains_key("email"));
        assert_eq!(p.source, "webhook");
    }

    #[test]
    fn passthrough_keeps_unmapped_keys() {
        let cfg = cfg_with(vec![("first_name", "firstName")], true, "{{first_name}}");
        let payload = serde_json::json!({ "first_name": "Sam", "email": "sam@x.com", "age": 30 });
        let p = build_profile_from_json(&cfg, &payload).unwrap();
        assert_eq!(p.values.get("firstName").unwrap(), "Sam");
        assert_eq!(p.values.get("email").unwrap(), "sam@x.com");
        assert_eq!(p.values.get("age").unwrap(), "30"); // number coerced to string
        assert_eq!(p.name, "Sam");
    }

    #[test]
    fn name_falls_back_when_template_empty() {
        let cfg = cfg_with(vec![("x", "x")], true, "{{missing}}");
        let payload = serde_json::json!({ "x": "hello" });
        let p = build_profile_from_json(&cfg, &payload).unwrap();
        assert_eq!(p.name, "hello"); // first value, since template rendered empty
    }

    #[test]
    fn rejects_non_object_payload() {
        let cfg = cfg_with(vec![], true, "n");
        assert!(build_profile_from_json(&cfg, &serde_json::json!([1, 2, 3])).is_none());
    }
}
