//! Inbound HTTP endpoint — a tiny loopback server a **Make HTTP module** (or an
//! n8n **HTTP Request** node, or the embedded AI agent) POSTs to, to push a
//! profile into Castline. Two named actions:
//!
//!   POST /api/create-profile  — body JSON becomes a brand-new profile.
//!   POST /api/update-profile  — body JSON is merged into an existing profile
//!                               matched by `name` (case-insensitive) or `email`.
//!
//! Auth is a bearer token, sent either as `Authorization: Bearer <token>` or a
//! `?token=<token>` query parameter. The server binds `127.0.0.1` only, so it's
//! not reachable from the internet without a tunnel — see the Connectors UI.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::Value;
use subtle::ConstantTimeEq;
use tauri::{AppHandle, Emitter, Manager};

use crate::connectors::build_profile_passthrough;
use crate::profiles::{self, ProfilesState};
use crate::settings::SettingsState;

// ─── Controller: start/stop the background server ────────────────────────────

struct RunningServer {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    port: u16,
}

#[derive(Default)]
pub struct HttpController {
    inner: Mutex<Option<RunningServer>>,
}

impl HttpController {
    /// Stop any running server, then start one on `port` when `enabled`.
    pub fn apply(&self, app: &AppHandle, enabled: bool, port: u16) {
        self.stop();
        if enabled {
            self.start(app, port);
        }
    }

    pub fn stop(&self) {
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
                    eprintln!("[castline] HTTP endpoint failed to bind 127.0.0.1:{port}: {e}");
                    return;
                }
            };
            eprintln!("[castline] HTTP endpoint listening on http://127.0.0.1:{port}/api/...");
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

// ─── Request handling ────────────────────────────────────────────────────────

fn respond(request: tiny_http::Request, status: u16, body: &str) {
    let header = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .expect("valid header");
    let response = tiny_http::Response::from_string(body).with_status_code(status).with_header(header);
    let _ = request.respond(response);
}

/// The path from the request URL, query stripped and trailing slash trimmed.
pub fn route_path(url: &str) -> &str {
    let path = url.split('?').next().unwrap_or("");
    path.trim_end_matches('/')
}

/// Pull the token from `Authorization: Bearer <t>` or the `?token=<t>` query.
fn request_token(request: &tiny_http::Request) -> String {
    for h in request.headers() {
        if h.field.equiv("Authorization") {
            let v = h.value.as_str();
            if let Some(t) = v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer ")) {
                return t.trim().to_string();
            }
        }
    }
    query_token(request.url()).unwrap_or_default()
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

// ─── Cross-origin guard (CSRF-to-localhost) ──────────────────────────────────

/// The host part of an `Origin` value (`scheme://host[:port]`), IPv6-aware.
fn origin_host(origin: &str) -> Option<&str> {
    let after_scheme = origin.split("://").nth(1)?;
    let host_port = after_scheme.split('/').next()?;
    let host = if let Some(rest) = host_port.strip_prefix('[') {
        rest.split(']').next()? // [::1] → ::1
    } else {
        host_port.split(':').next()?
    };
    Some(host)
}

/// True when a browser `Origin` header points anywhere other than loopback — a
/// cross-site call we must refuse (a malicious page fetching the endpoint). Real
/// clients (curl, the Make/n8n HTTP module, the embedded agent) send no `Origin`
/// and pass through; the token still gates them.
pub fn origin_is_foreign(request: &tiny_http::Request) -> bool {
    for h in request.headers() {
        if h.field.equiv("Origin") {
            return match origin_host(h.value.as_str().trim()) {
                Some(host) => !matches!(host, "127.0.0.1" | "localhost" | "::1"),
                None => true, // "null" / opaque / malformed → treat as foreign
            };
        }
    }
    false
}

// ─── Failed-auth throttle (brute-force guard) ────────────────────────────────

const MAX_FAILS: u32 = 10;
const THROTTLE_WINDOW: Duration = Duration::from_secs(30);

static AUTH_FAILS: AtomicU32 = AtomicU32::new(0);
static LAST_FAIL: Mutex<Option<Instant>> = Mutex::new(None);

/// Whether too many recent bad tokens should currently be refused (429). After
/// the cool-down window elapses the counter resets and requests flow again.
fn auth_throttled() -> bool {
    if AUTH_FAILS.load(Ordering::Relaxed) < MAX_FAILS {
        return false;
    }
    let mut last = LAST_FAIL.lock().unwrap();
    match *last {
        Some(t) if t.elapsed() < THROTTLE_WINDOW => true,
        _ => {
            AUTH_FAILS.store(0, Ordering::Relaxed);
            *last = None;
            false
        }
    }
}

fn record_auth_failure() {
    AUTH_FAILS.fetch_add(1, Ordering::Relaxed);
    *LAST_FAIL.lock().unwrap() = Some(Instant::now());
}

fn reset_auth_failures() {
    if AUTH_FAILS.load(Ordering::Relaxed) != 0 {
        AUTH_FAILS.store(0, Ordering::Relaxed);
        *LAST_FAIL.lock().unwrap() = None;
    }
}

/// Constant-time bearer-token check: latency doesn't leak the token byte-by-byte,
/// and an empty configured token never matches.
fn tokens_match(given: &str, expected: &str) -> bool {
    if expected.is_empty() {
        return false;
    }
    given.as_bytes().ct_eq(expected.as_bytes()).into()
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

fn handle_request(app: &AppHandle, mut request: tiny_http::Request) {
    if request.method() != &tiny_http::Method::Post {
        respond(request, 405, r#"{"ok":false,"error":"use POST"}"#);
        return;
    }

    // Refuse browser cross-site calls before anything else (CSRF-to-localhost).
    if origin_is_foreign(&request) {
        respond(request, 403, r#"{"ok":false,"error":"cross-origin requests are not allowed"}"#);
        return;
    }

    // Brute-force guard: too many recent bad tokens → cool off with a 429.
    if auth_throttled() {
        respond(request, 429, r#"{"ok":false,"error":"too many failed attempts, try again shortly"}"#);
        return;
    }

    let path = route_path(request.url()).to_string();
    let action = match path.as_str() {
        "/api/create-profile" => Action::Create,
        "/api/update-profile" => Action::Update,
        _ => {
            respond(request, 404, r#"{"ok":false,"error":"POST /api/create-profile or /api/update-profile"}"#);
            return;
        }
    };

    // Constant-time token check against the live settings (so a regenerated token
    // takes effect); repeated failures feed the throttle above.
    let expected = app.state::<SettingsState>().snapshot().http.token;
    let given = request_token(&request);
    if !tokens_match(&given, &expected) {
        record_auth_failure();
        respond(request, 401, r#"{"ok":false,"error":"invalid or missing token"}"#);
        return;
    }
    reset_auth_failures();

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
    let profile = match build_profile_passthrough(&payload) {
        Some(p) => p,
        None => {
            respond(request, 400, r#"{"ok":false,"error":"body must be a JSON object"}"#);
            return;
        }
    };

    let state = app.state::<ProfilesState>();
    match action {
        Action::Create => {
            let name = profile.name.clone();
            {
                let mut data = state.data.lock().unwrap();
                profiles::upsert_profile(&mut data, profile);
            }
            state.save();
            let _ = app.emit("profiles-changed", ());
            respond(request, 200, &ok_body("created", &name));
        }
        Action::Update => {
            let matched = {
                let mut data = state.data.lock().unwrap();
                profiles::enrich_existing(&mut data, &profile)
            };
            match matched {
                Some(name) => {
                    state.save();
                    let _ = app.emit("profiles-changed", ());
                    respond(request, 200, &ok_body("updated", &name));
                }
                None => respond(
                    request,
                    404,
                    r#"{"ok":false,"error":"no matching profile (by name or email)"}"#,
                ),
            }
        }
    }
}

enum Action {
    Create,
    Update,
}

fn ok_body(kind: &str, name: &str) -> String {
    let escaped = name.replace('\\', "\\\\").replace('"', "\\\"");
    format!(r#"{{"ok":true,"action":"{kind}","profile":"{escaped}"}}"#)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_path_strips_query_and_slash() {
        assert_eq!(route_path("/api/create-profile?token=abc"), "/api/create-profile");
        assert_eq!(route_path("/api/update-profile/"), "/api/update-profile");
        assert_eq!(route_path("/other"), "/other");
    }

    #[test]
    fn query_token_decodes() {
        assert_eq!(query_token("/api/create-profile?token=a%20b").as_deref(), Some("a b"));
        assert_eq!(query_token("/api/create-profile?foo=1&token=xyz").as_deref(), Some("xyz"));
        assert_eq!(query_token("/api/create-profile"), None);
    }

    #[test]
    fn origin_host_parses_ipv4_ipv6_and_ports() {
        assert_eq!(origin_host("http://127.0.0.1:8787"), Some("127.0.0.1"));
        assert_eq!(origin_host("https://localhost"), Some("localhost"));
        assert_eq!(origin_host("http://[::1]:8787"), Some("::1"));
        assert_eq!(origin_host("https://evil.com/path"), Some("evil.com"));
        // A look-alike host must NOT be mistaken for loopback.
        assert_eq!(origin_host("https://localhost.evil.com"), Some("localhost.evil.com"));
        assert_eq!(origin_host("null"), None);
    }

    #[test]
    fn tokens_match_is_exact_and_rejects_empty() {
        assert!(tokens_match("abc123", "abc123"));
        assert!(!tokens_match("abc123", "abc124"));
        assert!(!tokens_match("abc", "abc123")); // length mismatch
        // An unset (empty) configured token can never be matched.
        assert!(!tokens_match("", ""));
        assert!(!tokens_match("anything", ""));
    }
}
