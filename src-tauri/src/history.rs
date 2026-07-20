//! Recent-sends log — the last N outbound webhook sends with a payload
//! preview, shown in the Connectors tab. Once automations fire, "what exactly
//! did I send?" is the first debugging question; this answers it.
//!
//! Persisted to `<data_dir>/Castline/history.json` (best-effort, capped).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::library::{gen_id, now_iso};

/// How many sends we keep (newest first).
pub const HISTORY_CAP: usize = 50;
/// Payload previews are truncated to this many characters.
pub const PREVIEW_CAP: usize = 1500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendRecord {
    pub id: String,
    /// Local timestamp, `YYYY-MM-DDTHH:MM:SS`.
    pub ts: String,
    pub url: String,
    /// Human label of what was sent ("Item · Cold email", "Schedule · all profiles" …).
    #[serde(default)]
    pub label: String,
    /// HTTP status (0 = the request never got a response).
    #[serde(default)]
    pub status: u16,
    #[serde(default)]
    pub ok: bool,
    /// Transport error text when `status == 0`.
    #[serde(default)]
    pub error: String,
    /// Truncated payload (what left the app).
    #[serde(default)]
    pub preview: String,
}

/// Redact secret-bearing query parameters (e.g. the endpoint's own `?token=…`)
/// so a logged URL never persists an auth secret to disk. The parameter name is
/// kept for context; only its value is masked.
pub fn redact_url(url: &str) -> String {
    let Some((base, query)) = url.split_once('?') else {
        return url.to_string();
    };
    const SENSITIVE: [&str; 6] = ["token", "key", "apikey", "api_key", "secret", "access_token"];
    let redacted: Vec<String> = query
        .split('&')
        .map(|pair| {
            let name = pair.split('=').next().unwrap_or("");
            if SENSITIVE.iter().any(|s| name.eq_ignore_ascii_case(s)) {
                format!("{name}=…")
            } else {
                pair.to_string()
            }
        })
        .collect();
    format!("{base}?{}", redacted.join("&"))
}

/// Build a record from a send's inputs + outcome.
pub fn make_record(url: &str, label: &str, body: &str, outcome: &Result<u16, String>) -> SendRecord {
    let mut preview: String = body.chars().take(PREVIEW_CAP).collect();
    if preview.len() < body.len() {
        preview.push('…');
    }
    let (status, ok, error) = match outcome {
        Ok(code) => (*code, *code < 300, String::new()),
        Err(e) => (0, false, e.clone()),
    };
    SendRecord {
        id: gen_id(),
        ts: now_iso(),
        url: redact_url(url),
        label: label.to_string(),
        status,
        ok,
        error,
        preview,
    }
}

/// Prepend a record, keeping at most `HISTORY_CAP` entries.
pub fn push(list: &mut Vec<SendRecord>, record: SendRecord) {
    list.insert(0, record);
    list.truncate(HISTORY_CAP);
}

/// Tauri-managed history + the JSON file it persists to.
pub struct HistoryState {
    pub data: Mutex<Vec<SendRecord>>,
    pub path: PathBuf,
}

impl HistoryState {
    pub fn load(path: PathBuf, warnings: &mut Vec<String>) -> Self {
        let data = match crate::storage::load_json::<Vec<SendRecord>>(&path) {
            crate::storage::LoadedStore::Parsed(d) => d,
            crate::storage::LoadedStore::Corrupt { backup } => {
                warnings.push(crate::storage::corrupt_warning("history.json", &backup));
                Vec::new()
            }
            crate::storage::LoadedStore::Missing => Vec::new(),
        };
        HistoryState { data: Mutex::new(data), path }
    }

    pub fn save(&self) {
        if let Ok(data) = self.data.lock() {
            if let Ok(json) = serde_json::to_string_pretty(&*data) {
                let _ = crate::storage::write_atomic(&self.path, &json);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_captures_outcome_and_truncates_preview() {
        let ok = make_record("http://x/hook", "Item · Cold email", "{\"a\":1}", &Ok(200));
        assert!(ok.ok);
        assert_eq!(ok.status, 200);
        assert_eq!(ok.preview, "{\"a\":1}");

        let failed = make_record("http://x/hook", "Test", "{}", &Ok(500));
        assert!(!failed.ok);
        assert_eq!(failed.status, 500);

        let err = make_record("http://x/hook", "Test", "{}", &Err("timed out".into()));
        assert!(!err.ok);
        assert_eq!(err.status, 0);
        assert_eq!(err.error, "timed out");

        let long_body = "x".repeat(PREVIEW_CAP + 100);
        let trunc = make_record("u", "l", &long_body, &Ok(200));
        assert!(trunc.preview.chars().count() == PREVIEW_CAP + 1); // + ellipsis
        assert!(trunc.preview.ends_with('…'));
    }

    #[test]
    fn redact_url_masks_secret_query_params_only() {
        // The endpoint's own test URL must not persist its bearer token.
        assert_eq!(
            redact_url("http://127.0.0.1:8787/api/create-profile?token=deadbeefsecret"),
            "http://127.0.0.1:8787/api/create-profile?token=…"
        );
        // Non-secret params survive for debugging; secrets among them are masked.
        assert_eq!(
            redact_url("https://hook.make.com/x?foo=1&api_key=abc&bar=2"),
            "https://hook.make.com/x?foo=1&api_key=…&bar=2"
        );
        // No query string → untouched (Make/n8n secrets live in the path; those
        // are user-configured connector identifiers shown in the UI by design).
        assert_eq!(redact_url("https://hook.make.com/abc123"), "https://hook.make.com/abc123");
    }

    #[test]
    fn make_record_stores_redacted_url() {
        let r = make_record("http://127.0.0.1:8787/api/update-profile?token=supersecret", "Test", "{}", &Ok(200));
        assert!(!r.url.contains("supersecret"));
        assert!(r.url.contains("token=…"));
    }

    #[test]
    fn push_caps_newest_first_and_roundtrips() {
        let mut list = Vec::new();
        for i in 0..(HISTORY_CAP + 10) {
            push(&mut list, make_record("u", &format!("send {i}"), "{}", &Ok(200)));
        }
        assert_eq!(list.len(), HISTORY_CAP);
        assert_eq!(list[0].label, format!("send {}", HISTORY_CAP + 9)); // newest first

        // Persistence round-trip.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");
        let state = HistoryState { data: Mutex::new(list), path: path.clone() };
        state.save();
        let reloaded = HistoryState::load(path, &mut Vec::new());
        let data = reloaded.data.lock().unwrap();
        assert_eq!(data.len(), HISTORY_CAP);
        assert_eq!(data[0].label, format!("send {}", HISTORY_CAP + 9));
    }
}
