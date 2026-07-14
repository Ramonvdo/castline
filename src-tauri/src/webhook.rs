//! Incoming-webhook receiver + the shared "JSON payload → profile" routine.
//!
//! One local HTTP server (bound to `127.0.0.1`) routes several named webhooks by
//! path: `POST /hook/<path>?token=<secret>`. Each webhook has its own token and
//! field mapping. A desktop app on localhost isn't reachable from the public
//! internet, so front it with a tunnel (ngrok / Cloudflare) or a relay
//! (Make / n8n / Zapier). The same mapping powers the "Paste JSON" importer.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

use crate::profiles::{self, Profile, ProfilesState};
use crate::settings::{SettingsState, Webhook};

// ─── Shared mapping: JSON → Profile ──────────────────────────────────────────

fn val_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn render_name(template: &str, obj: &serde_json::Map<String, Value>) -> String {
    let mut out = template.to_string();
    for (k, v) in obj {
        out = out.replace(&format!("{{{{{k}}}}}"), &val_to_string(v));
    }
    while let Some(start) = out.find("{{") {
        if let Some(end) = out[start..].find("}}") {
            out.replace_range(start..start + end + 2, "");
        } else {
            break;
        }
    }
    out.trim().to_string()
}

/// Turn an incoming JSON payload into a `Profile` using a webhook's field mapping.
/// Returns `None` when the payload isn't a JSON object.
pub fn build_profile_from_json(wh: &Webhook, payload: &Value) -> Option<Profile> {
    let obj = payload.as_object()?;
    let mut values = std::collections::BTreeMap::new();

    let mapped_keys: std::collections::HashSet<&str> =
        wh.mappings.iter().map(|m| m.from.as_str()).collect();
    for m in &wh.mappings {
        if let Some(v) = obj.get(&m.from) {
            let to = m.to.trim();
            if !to.is_empty() {
                values.insert(to.to_string(), val_to_string(v));
            }
        }
    }
    if wh.passthrough {
        for (k, v) in obj {
            if !mapped_keys.contains(k.as_str()) {
                values.entry(k.clone()).or_insert_with(|| val_to_string(v));
            }
        }
    }

    let mut name = render_name(&wh.name_template, obj);
    if name.is_empty() {
        name = values.values().next().cloned().unwrap_or_else(|| "Webhook profile".into());
    }
    Some(Profile { id: String::new(), name, values, source: "webhook".into() })
}

/// Build a profile from pasted JSON with no mapping (every key passes through).
pub fn build_profile_passthrough(payload: &Value) -> Option<Profile> {
    let wh = Webhook {
        id: String::new(),
        name: String::new(),
        path: String::new(),
        token: String::new(),
        name_template: String::new(),
        mappings: vec![],
        passthrough: true,
    };
    let mut p = build_profile_from_json(&wh, payload)?;
    p.source = "import".into();
    Some(p)
}

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

#[derive(Default)]
pub struct WebhookController {
    inner: Mutex<Option<RunningServer>>,
}

impl WebhookController {
    /// Stop any running server and, if `enabled`, start one on `port`.
    pub fn apply(&self, app: &AppHandle, enabled: bool, port: u16) {
        self.stop();
        if enabled {
            self.start(app, port);
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

    fn start(&self, app: &AppHandle, port: u16) {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let app = app.clone();

        let handle = std::thread::spawn(move || {
            let server = match tiny_http::Server::http(("127.0.0.1", port)) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[castline] webhook receiver failed to bind 127.0.0.1:{port}: {e}");
                    return;
                }
            };
            eprintln!("[castline] webhook receiver listening on http://127.0.0.1:{port}/hook/<path>");
            while !stop_thread.load(Ordering::Relaxed) {
                match server.recv_timeout(Duration::from_millis(500)) {
                    Ok(Some(request)) => handle_request(&app, request),
                    Ok(None) => {}
                    Err(_) => break,
                }
            }
        });

        *self.inner.lock().unwrap() = Some(RunningServer { stop, handle: Some(handle), port });
    }

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

/// The `<path>` from `/hook/<path>` (query stripped, trailing slash trimmed).
fn hook_path(url: &str) -> Option<String> {
    let path = url.split('?').next().unwrap_or("");
    let rest = path.strip_prefix("/hook/")?;
    let rest = rest.trim_end_matches('/');
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

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
    let response = tiny_http::Response::from_string(body).with_status_code(status).with_header(header);
    let _ = request.respond(response);
}

fn handle_request(app: &AppHandle, mut request: tiny_http::Request) {
    if request.method() != &tiny_http::Method::Post {
        respond(request, 405, r#"{"ok":false,"error":"use POST"}"#);
        return;
    }
    let slug = match hook_path(request.url()) {
        Some(s) => s,
        None => {
            respond(request, 404, r#"{"ok":false,"error":"POST to /hook/<path>"}"#);
            return;
        }
    };
    let token = query_token(request.url()).unwrap_or_default();

    let receiver = app.state::<SettingsState>().snapshot().receiver;
    let webhook = receiver.webhooks.into_iter().find(|w| w.path == slug);
    let webhook = match webhook {
        Some(w) => w,
        None => {
            respond(request, 404, r#"{"ok":false,"error":"no webhook at that path"}"#);
            return;
        }
    };
    if webhook.token.is_empty() || token != webhook.token {
        respond(request, 401, r#"{"ok":false,"error":"invalid or missing token"}"#);
        return;
    }

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

    match build_profile_from_json(&webhook, &payload) {
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
    use crate::settings::{FieldMap, Webhook};

    fn wh(mappings: Vec<(&str, &str)>, passthrough: bool, name_tpl: &str) -> Webhook {
        Webhook {
            id: "w1".into(),
            name: "Test".into(),
            path: "test".into(),
            token: "secret".into(),
            name_template: name_tpl.into(),
            mappings: mappings.into_iter().map(|(f, t)| FieldMap { from: f.into(), to: t.into() }).collect(),
            passthrough,
        }
    }

    #[test]
    fn maps_fields_and_builds_name() {
        let cfg = wh(vec![("first_name", "firstName"), ("last_name", "lastName")], false, "{{first_name}} {{last_name}}");
        let payload = serde_json::json!({ "first_name": "Sam", "last_name": "Rivera", "email": "sam@x.com" });
        let p = build_profile_from_json(&cfg, &payload).unwrap();
        assert_eq!(p.name, "Sam Rivera");
        assert_eq!(p.values.get("firstName").unwrap(), "Sam");
        assert!(!p.values.contains_key("email"));
    }

    #[test]
    fn passthrough_keeps_unmapped_keys() {
        let cfg = wh(vec![("first_name", "firstName")], true, "{{first_name}}");
        let payload = serde_json::json!({ "first_name": "Sam", "email": "sam@x.com", "age": 30 });
        let p = build_profile_from_json(&cfg, &payload).unwrap();
        assert_eq!(p.values.get("email").unwrap(), "sam@x.com");
        assert_eq!(p.values.get("age").unwrap(), "30");
    }

    #[test]
    fn passthrough_import_builds_from_any_object() {
        let p = build_profile_passthrough(&serde_json::json!({ "x": "hello" })).unwrap();
        assert_eq!(p.values.get("x").unwrap(), "hello");
        assert_eq!(p.source, "import");
    }

    #[test]
    fn hook_path_extracts_slug() {
        assert_eq!(hook_path("/hook/calendly?token=abc").as_deref(), Some("calendly"));
        assert_eq!(hook_path("/hook/typeform/").as_deref(), Some("typeform"));
        assert_eq!(hook_path("/hook/").as_deref(), None);
        assert_eq!(hook_path("/other").as_deref(), None);
    }

    #[test]
    fn rejects_non_object_payload() {
        let cfg = wh(vec![], true, "n");
        assert!(build_profile_from_json(&cfg, &serde_json::json!([1, 2, 3])).is_none());
    }
}
