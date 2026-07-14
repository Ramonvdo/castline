//! Outbound connectors + the JSON→profile passthrough used by the paste importer
//! and "create from connector".
//!
//! Castline POSTs a JSON body to a pasted Make/n8n webhook URL and reads the
//! response the scenario returns (Make "Webhook response" / n8n "Respond to
//! Webhook"). This is a single request/response round-trip on a connection
//! Castline opens itself — no inbound server, no tunnel, no open port.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::profiles::Profile;

/// Result of an outbound connector call.
#[derive(Debug, Clone, Serialize)]
pub struct ConnectorResult {
    pub status: u16,
    pub body: String,
}

/// POST `body` (JSON) to `url` and return the response status + body. Non-2xx
/// responses are returned as a result too (so the UI can show the status/body).
pub fn connector_send(url: &str, body: &str) -> Result<ConnectorResult, String> {
    if url.trim().is_empty() {
        return Err("connector URL is empty".into());
    }
    let req = ureq::post(url)
        .timeout(Duration::from_secs(20))
        .set("Content-Type", "application/json");
    match req.send_string(body) {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.into_string().unwrap_or_default();
            Ok(ConnectorResult { status, body: text })
        }
        // 4xx/5xx still carry a response body worth showing.
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            Ok(ConnectorResult { status: code, body: text })
        }
        Err(e) => Err(e.to_string()),
    }
}

// ─── JSON → profile (passthrough) ────────────────────────────────────────────

fn val_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn str_field(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Some(s) = obj.get(*k).and_then(|v| v.as_str()) {
            if !s.trim().is_empty() {
                return Some(s.trim().to_string());
            }
        }
    }
    None
}

/// A readable profile name from common keys, else the first non-empty value.
fn derive_name(obj: &serde_json::Map<String, Value>, values: &BTreeMap<String, String>) -> String {
    if let Some(full) = str_field(obj, &["name", "full_name", "fullName"]) {
        return full;
    }
    let first = str_field(obj, &["first_name", "firstName", "firstname"]);
    let last = str_field(obj, &["last_name", "lastName", "lastname"]);
    if let Some(f) = first {
        return match last {
            Some(l) => format!("{f} {l}"),
            None => f,
        };
    }
    values.values().find(|v| !v.trim().is_empty()).cloned().unwrap_or_else(|| "Imported profile".into())
}

/// Build a profile from a JSON object: every key becomes a variable of the same
/// name (mapping lives in Make/n8n, not here). Returns `None` if not an object.
pub fn build_profile_passthrough(payload: &Value) -> Option<Profile> {
    let obj = payload.as_object()?;
    let mut values = BTreeMap::new();
    for (k, v) in obj {
        values.insert(k.clone(), val_to_string(v));
    }
    let name = derive_name(obj, &values);
    Some(Profile { id: String::new(), name, values, source: "import".into() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_keeps_all_keys() {
        let p = build_profile_passthrough(&serde_json::json!({
            "first_name": "Sam", "last_name": "Rivera", "email": "sam@x.com", "age": 30
        }))
        .unwrap();
        assert_eq!(p.name, "Sam Rivera");
        assert_eq!(p.values.get("email").unwrap(), "sam@x.com");
        assert_eq!(p.values.get("age").unwrap(), "30");
        assert_eq!(p.source, "import");
    }

    #[test]
    fn name_falls_back_to_first_value() {
        let p = build_profile_passthrough(&serde_json::json!({ "company": "Acme" })).unwrap();
        assert_eq!(p.name, "Acme");
    }

    #[test]
    fn rejects_non_object() {
        assert!(build_profile_passthrough(&serde_json::json!([1, 2, 3])).is_none());
    }

    #[test]
    fn connector_send_posts_and_reads_response() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        // A tiny stand-in for a Make "Custom webhook + Webhook response": accept
        // one request and reply with a JSON body.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            // Fully read the request (headers + Content-Length body) — otherwise
            // unread bytes force an RST on close (Windows) and drop the response.
            let mut data = Vec::new();
            let mut buf = [0u8; 256];
            loop {
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        data.extend_from_slice(&buf[..n]);
                        let s = String::from_utf8_lossy(&data);
                        if let Some(hp) = s.find("\r\n\r\n") {
                            let clen = s[..hp]
                                .lines()
                                .find_map(|l| {
                                    let l = l.to_ascii_lowercase();
                                    l.strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0))
                                })
                                .unwrap_or(0);
                            if data.len() >= hp + 4 + clen {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            let body = r#"{"firstName":"Sam","company":"Acme"}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
        });

        let url = format!("http://{}/", addr);
        let res = connector_send(&url, r#"{"email":"sam@x.com"}"#).unwrap();
        handle.join().unwrap();

        assert_eq!(res.status, 200);
        // The response body feeds build_profile_passthrough (enrich / create).
        let profile = build_profile_passthrough(&serde_json::from_str(&res.body).unwrap()).unwrap();
        assert_eq!(profile.values.get("firstName").unwrap(), "Sam");
        assert_eq!(profile.values.get("company").unwrap(), "Acme");
    }
}
