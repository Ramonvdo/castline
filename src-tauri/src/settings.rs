//! Persistent app settings (`<config_dir>/Castline/settings.json`): appearance
//! (accent colour) and the incoming-webhook configuration (port, secret token,
//! and the field-mapping that turns an inbound JSON payload into a profile).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

/// One rule: copy incoming JSON key `from` into profile variable `to`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMap {
    pub from: String,
    pub to: String,
}

/// Incoming-webhook receiver configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_port")]
    pub port: u16,
    /// Secret required as `?token=…`; generated when the receiver is first enabled.
    #[serde(default)]
    pub token: String,
    /// Template for the new profile's display name, using `{{incoming_key}}`
    /// placeholders, e.g. `{{first_name}} {{last_name}}`.
    #[serde(default = "default_name_template")]
    pub name_template: String,
    #[serde(default)]
    pub mappings: Vec<FieldMap>,
    /// When true, any incoming key without an explicit mapping is copied into a
    /// variable of the same name.
    #[serde(default = "default_true")]
    pub passthrough: bool,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_port(),
            token: String::new(),
            name_template: default_name_template(),
            mappings: Vec::new(),
            passthrough: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_accent")]
    pub accent: String,
    /// Kept for forward-compatibility; Castline ships a single dark theme.
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub webhook: WebhookConfig,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            accent: default_accent(),
            theme: default_theme(),
            webhook: WebhookConfig::default(),
        }
    }
}

fn default_port() -> u16 {
    8787
}
fn default_name_template() -> String {
    "{{first_name}} {{last_name}}".into()
}
fn default_accent() -> String {
    "#4f8cff".into()
}
fn default_theme() -> String {
    "dark".into()
}
fn default_true() -> bool {
    true
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
