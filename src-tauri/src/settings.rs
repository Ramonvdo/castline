//! Persistent app settings (`<config_dir>/Castline/settings.json`): the list of
//! outbound **connectors** — Make / n8n (or any) webhook URLs Castline POSTs to
//! and reads a response from. No inbound server, so no tunnel or open port.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::library::gen_id;

/// An outbound connector: a pasted webhook URL Castline POSTs profile data to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connector {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
}

/// The inbound HTTP endpoint: a loopback server a Make/n8n HTTP module POSTs to
/// (`/api/create-profile`, `/api/update-profile`). Token-gated, off by default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub token: String,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self { enabled: false, port: default_port(), token: String::new() }
    }
}

/// Settings for the embedded AI agent (the "Agent" tab running Claude Code).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiConfig {
    /// Explicit path to the `claude` binary; empty = auto-detect on PATH.
    #[serde(default)]
    pub claude_path: String,
    /// Extra arguments appended to the `claude` invocation.
    #[serde(default)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Reserved for forward-compat; Castline ships one fixed dark theme.
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub connectors: Vec<Connector>,
    #[serde(default)]
    pub http: HttpConfig,
    #[serde(default)]
    pub ai: AiConfig,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            connectors: Vec::new(),
            http: HttpConfig::default(),
            ai: AiConfig::default(),
        }
    }
}

fn default_theme() -> String {
    "dark".into()
}

fn default_port() -> u16 {
    8787
}

/// Give the endpoint a long random bearer token when it's first switched on.
pub fn ensure_http_token(http: &mut HttpConfig) {
    if http.token.trim().is_empty() {
        http.token = format!("{}{}", gen_id(), gen_id());
    }
}

/// Give every connector a stable id (called whenever connectors are saved).
pub fn normalize_connectors(connectors: &mut [Connector]) {
    for c in connectors.iter_mut() {
        if c.id.trim().is_empty() {
            c.id = gen_id();
        }
    }
}

/// `<config_dir>/Castline`
pub fn app_config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("Castline")
}

/// `<data_dir>/Castline` (where library.json + profiles.json live).
pub fn app_data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("Castline")
}

fn settings_path() -> PathBuf {
    app_config_dir().join("settings.json")
}

pub struct SettingsState {
    pub data: Mutex<AppSettings>,
}

impl SettingsState {
    pub fn load() -> Self {
        let data = match fs::read_to_string(settings_path()) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => {
                let d = AppSettings::default();
                let _ = save_to_disk(&d);
                d
            }
        };
        SettingsState { data: Mutex::new(data) }
    }

    pub fn save(&self) {
        if let Ok(data) = self.data.lock() {
            let _ = save_to_disk(&data);
        }
    }

    pub fn snapshot(&self) -> AppSettings {
        self.data.lock().unwrap().clone()
    }
}

fn save_to_disk(settings: &AppSettings) -> std::io::Result<()> {
    let dir = app_config_dir();
    fs::create_dir_all(&dir)?;
    let text = serde_json::to_string_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(settings_path(), text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_fills_ids() {
        let mut cs = vec![
            Connector { id: String::new(), name: "Enrich".into(), url: "https://hook.make.com/x".into() },
            Connector { id: "keep".into(), name: "B".into(), url: "https://n8n/y".into() },
        ];
        normalize_connectors(&mut cs);
        assert!(!cs[0].id.is_empty());
        assert_eq!(cs[1].id, "keep");
    }

    #[test]
    fn old_settings_with_receiver_still_loads() {
        // A settings.json from the inbound-receiver era must still deserialize;
        // the unknown `receiver` field is ignored and connectors default to [].
        let json = r#"{ "theme": "dark", "receiver": { "enabled": true, "port": 8787, "webhooks": [] } }"#;
        let s: AppSettings = serde_json::from_str(json).unwrap();
        assert_eq!(s.theme, "dark");
        assert!(s.connectors.is_empty());
        // New http/ai sections default in cleanly for old files.
        assert!(!s.http.enabled);
        assert_eq!(s.http.port, 8787);
        assert!(s.http.token.is_empty());
        assert!(s.ai.claude_path.is_empty());
    }

    #[test]
    fn ensure_http_token_fills_once() {
        let mut http = HttpConfig::default();
        assert!(http.token.is_empty());
        ensure_http_token(&mut http);
        let first = http.token.clone();
        assert!(first.len() > 16);
        // Idempotent: a second call keeps the existing token.
        ensure_http_token(&mut http);
        assert_eq!(http.token, first);
    }
}
