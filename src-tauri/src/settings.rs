//! Persistent app settings (`<config_dir>/Castline/settings.json`): the incoming
//! webhook receiver (a shared local HTTP server) and the list of named webhooks
//! it routes. Each webhook has its own path, secret token and field mapping, so
//! one receiver can accept payloads from several sources (Calendly, Typeform, …).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::library::gen_id;

/// One rule: copy incoming JSON key `from` into profile variable `to`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMap {
    pub from: String,
    pub to: String,
}

/// A named webhook endpoint served at `/hook/<path>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// URL slug: the endpoint is `/hook/<path>`.
    #[serde(default)]
    pub path: String,
    /// Secret required as `?token=…` (generated when the webhook is created).
    #[serde(default)]
    pub token: String,
    /// Template for the new profile's name, using `{{incoming_key}}` placeholders.
    #[serde(default = "default_name_template")]
    pub name_template: String,
    #[serde(default)]
    pub mappings: Vec<FieldMap>,
    /// When true, unmapped incoming keys become variables of the same name.
    #[serde(default = "default_true")]
    pub passthrough: bool,
}

/// The shared local receiver + the webhooks it routes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiverConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub webhooks: Vec<Webhook>,
}

impl Default for ReceiverConfig {
    fn default() -> Self {
        Self { enabled: false, port: default_port(), webhooks: Vec::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Reserved for forward-compat; Castline ships one fixed dark theme.
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub receiver: ReceiverConfig,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self { theme: default_theme(), receiver: ReceiverConfig::default() }
    }
}

fn default_port() -> u16 {
    8787
}
fn default_name_template() -> String {
    "{{first_name}} {{last_name}}".into()
}
fn default_theme() -> String {
    "dark".into()
}
fn default_true() -> bool {
    true
}

/// A url-safe slug from a webhook name (fallback when the user leaves path blank).
pub fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in s.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Fill in ids, tokens and unique paths for any webhook missing them. Called
/// whenever the receiver config is saved so every webhook is addressable + secret.
pub fn normalize_receiver(cfg: &mut ReceiverConfig) {
    if cfg.port == 0 {
        cfg.port = default_port();
    }
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    for wh in &mut cfg.webhooks {
        if wh.id.trim().is_empty() {
            wh.id = gen_id();
        }
        if wh.token.trim().is_empty() {
            wh.token = format!("{}{}", gen_id(), gen_id());
        }
        if wh.path.trim().is_empty() {
            wh.path = slugify(&wh.name);
            if wh.path.is_empty() {
                wh.path = wh.id.clone();
            }
        } else {
            wh.path = slugify(&wh.path);
        }
        // Ensure uniqueness of the routing path.
        let mut candidate = wh.path.clone();
        let mut n = 2;
        while used.contains(&candidate) {
            candidate = format!("{}-{}", wh.path, n);
            n += 1;
        }
        wh.path = candidate.clone();
        used.insert(candidate);
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
    fn slugify_makes_url_safe_paths() {
        assert_eq!(slugify("Calendly Booking!"), "calendly-booking");
        assert_eq!(slugify("  Type form  "), "type-form");
        assert_eq!(slugify("***"), "");
    }

    #[test]
    fn normalize_fills_ids_tokens_and_unique_paths() {
        let mut cfg = ReceiverConfig {
            enabled: true,
            port: 0,
            webhooks: vec![
                Webhook { id: String::new(), name: "Calendly".into(), path: String::new(), token: String::new(), name_template: default_name_template(), mappings: vec![], passthrough: true },
                Webhook { id: String::new(), name: "Calendly".into(), path: String::new(), token: String::new(), name_template: default_name_template(), mappings: vec![], passthrough: true },
            ],
        };
        normalize_receiver(&mut cfg);
        assert_eq!(cfg.port, 8787);
        assert!(cfg.webhooks.iter().all(|w| !w.id.is_empty() && !w.token.is_empty()));
        // Duplicate names must not collide on path.
        assert_ne!(cfg.webhooks[0].path, cfg.webhooks[1].path);
    }
}
