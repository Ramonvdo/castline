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

/// The "Castline AI" enrich workflow — one OpenRouter call that fills profile
/// variables (optionally with live web research via the `:online` suffix).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    /// Append `:online` to the model so OpenRouter runs live web search.
    /// Defaults ON — without research the model invents company facts, which
    /// is exactly the "weird icebreaker" failure mode. The enrich dialogs
    /// auto-check their box from this; users untick per run (or here).
    #[serde(default = "default_true")]
    pub web_search: bool,
    /// Tone of voice for generated text values. Prefilled with a suggested
    /// default on first run (like the model field) so setup is fast — but it's
    /// just text: clear it and no tone is applied at all. Only used when the
    /// enrich dialog's "Tone of voice" checkbox is ticked.
    #[serde(default = "default_tone")]
    pub tone: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self { api_key: String::new(), model: default_model(), web_search: true, tone: default_tone() }
    }
}

fn default_model() -> String {
    "google/gemini-2.5-flash".into()
}

/// The suggested starter tone — a prefill, not a fallback.
pub fn default_tone() -> String {
    "Casual, charismatic, original phrasing. Straight to the point. Never use em dashes (—); \
     use commas or periods instead. No clichés, no corporate filler, no AI-sounding hedging."
        .into()
}

/// A recurring job: POST all profiles / one item / one folder to a connector,
/// or back the data up to a local folder — every day/week/month. Runs while
/// the app is open. Missed runs are **skipped** (cadence re-anchors at launch)
/// unless `catch_up` is set, in which case at most ONE catch-up run fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    #[serde(default)]
    pub id: String,
    /// "profiles" (all profiles) | "item" (one library item) |
    /// "folder" (all items of one folder) | "backup" (local file backup).
    #[serde(default = "default_schedule_kind")]
    pub kind: String,
    /// The library item to send when `kind == "item"`.
    #[serde(default)]
    pub item_id: String,
    /// The folder to send when `kind == "folder"`.
    #[serde(default)]
    pub folder_id: String,
    /// Target directory when `kind == "backup"`.
    #[serde(default)]
    pub dir: String,
    /// Which connector receives the payload (unused for "backup").
    #[serde(default)]
    pub connector_id: String,
    /// "day" | "week" | "month".
    #[serde(default = "default_every")]
    pub every: String,
    /// Unix seconds of the last run (0 = never).
    #[serde(default)]
    pub last_run: i64,
    /// Run once at launch if a cadence was missed (off = skip missed runs).
    #[serde(default)]
    pub catch_up: bool,
}

fn default_schedule_kind() -> String {
    "profiles".into()
}
fn default_every() -> String {
    "week".into()
}

/// Seconds between runs for a schedule's `every` value.
pub fn every_secs(every: &str) -> i64 {
    match every {
        "day" => 86_400,
        "month" => 30 * 86_400,
        _ => 7 * 86_400, // week (default)
    }
}

/// Whether a schedule is due at `now` (unix secs).
pub fn schedule_due(s: &Schedule, now: i64) -> bool {
    now - s.last_run >= every_secs(&s.every)
}

/// Give every schedule a stable id, and anchor brand-new ones at `now` so the
/// first send happens after one full cadence (use "Run now" for immediately).
pub fn normalize_schedules(schedules: &mut [Schedule], now: i64) {
    for s in schedules.iter_mut() {
        if s.id.trim().is_empty() {
            s.id = gen_id();
            if s.last_run == 0 {
                s.last_run = now;
            }
        }
    }
}

/// Launch-time reconciliation: schedules that missed their window while the
/// app was closed get re-anchored at `now` WITHOUT sending — unless they opted
/// into `catch_up` (those stay due and the ticker fires them exactly once).
/// Returns whether anything changed (caller persists).
pub fn reconcile_missed(schedules: &mut [Schedule], now: i64) -> bool {
    let mut changed = false;
    for s in schedules.iter_mut() {
        if !s.catch_up && schedule_due(s, now) {
            s.last_run = now;
            changed = true;
        }
    }
    changed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Reserved for forward-compat; Castline ships one fixed dark theme.
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Launch Castline when the user logs in (default on; Settings toggle).
    #[serde(default = "default_true")]
    pub autostart: bool,
    #[serde(default)]
    pub connectors: Vec<Connector>,
    #[serde(default)]
    pub http: HttpConfig,
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub schedules: Vec<Schedule>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            autostart: true,
            connectors: Vec::new(),
            http: HttpConfig::default(),
            ai: AiConfig::default(),
            llm: LlmConfig::default(),
            schedules: Vec::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "dark".into()
}

fn default_port() -> u16 {
    8787
}

/// A bearer token minted from the OS CSPRNG: 32 random bytes (256 bits) as hex.
/// This is an authentication secret, so it must NOT come from `gen_id()` (which
/// is a time+counter id — predictable from the wall clock).
pub fn secure_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("OS RNG unavailable");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A modern `secure_token()` is 64 hex chars. Anything shorter is either empty
/// or a legacy weak token (two `gen_id()`s) and must be rotated.
const MIN_TOKEN_LEN: usize = 48;

/// Ensure the endpoint has a strong bearer token: mint one when it's first
/// switched on, and transparently rotate any legacy weak/short token once.
pub fn ensure_http_token(http: &mut HttpConfig) {
    if http.token.trim().len() < MIN_TOKEN_LEN {
        http.token = secure_token();
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

    fn sched(every: &str, last_run: i64, catch_up: bool) -> Schedule {
        Schedule {
            id: "s1".into(),
            kind: "profiles".into(),
            item_id: String::new(),
            folder_id: String::new(),
            dir: String::new(),
            connector_id: "c1".into(),
            every: every.into(),
            last_run,
            catch_up,
        }
    }

    #[test]
    fn schedule_due_logic() {
        let mut s = sched("day", 0, false);
        // Never run (last_run = 0) → due immediately at any real clock time.
        let now = 1_700_000_000;
        assert!(schedule_due(&s, now));
        // Ran 1h ago on a daily cadence → not due.
        s.last_run = now;
        assert!(!schedule_due(&s, now + 3_600));
        // 25h ago → due.
        assert!(schedule_due(&s, now + 25 * 3_600));
        // Weekly default for unknown values.
        s.every = "fortnight".into();
        assert!(!schedule_due(&s, now + 6 * 86_400));
        assert!(schedule_due(&s, now + 8 * 86_400));

        // Old settings without llm/schedules load cleanly; autostart defaults on.
        let json = r#"{ "theme": "dark" }"#;
        let cfg: AppSettings = serde_json::from_str(json).unwrap();
        assert!(cfg.schedules.is_empty());
        assert!(cfg.llm.api_key.is_empty());
        assert_eq!(cfg.llm.model, "google/gemini-2.5-flash");
        // Web research defaults ON (no research = invented company facts);
        // an explicit false still round-trips.
        assert!(cfg.llm.web_search);
        let no_web: AppSettings =
            serde_json::from_str(r#"{ "llm": { "web_search": false } }"#).unwrap();
        assert!(!no_web.llm.web_search);
        assert!(cfg.autostart);
        // Tone arrives PREFILLED (a starter suggestion, not a hidden fallback)…
        assert!(cfg.llm.tone.contains("em dashes"));
        // …but a deliberately cleared tone stays cleared.
        let cleared: AppSettings =
            serde_json::from_str(r#"{ "llm": { "tone": "" } }"#).unwrap();
        assert!(cleared.llm.tone.is_empty());
    }

    #[test]
    fn missed_schedules_reanchor_unless_catch_up() {
        let now = 1_700_000_000;
        let week_ago = now - 8 * 86_400;
        let mut list = vec![
            sched("day", week_ago, false),  // missed, no catch-up → re-anchor
            sched("day", week_ago, true),   // missed, catch-up → left due
            sched("week", now - 3_600, false), // not due → untouched
        ];
        assert!(reconcile_missed(&mut list, now));
        assert_eq!(list[0].last_run, now); // skipped, cadence restarts
        assert_eq!(list[1].last_run, week_ago); // still due for ONE catch-up run
        assert!(schedule_due(&list[1], now));
        assert_eq!(list[2].last_run, now - 3_600);

        // Nothing due → no change reported.
        assert!(!reconcile_missed(&mut list, now));

        // New schedules get anchored at save time (first send after one cadence).
        let mut fresh = vec![sched("week", 0, false)];
        fresh[0].id = String::new();
        normalize_schedules(&mut fresh, now);
        assert!(!fresh[0].id.is_empty());
        assert_eq!(fresh[0].last_run, now);
        assert!(!schedule_due(&fresh[0], now + 60));
    }

    #[test]
    fn ensure_http_token_fills_once() {
        let mut http = HttpConfig::default();
        assert!(http.token.is_empty());
        ensure_http_token(&mut http);
        let first = http.token.clone();
        // 32 random bytes → 64 hex chars, and hex-only.
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        // Idempotent: a second call keeps the existing (already-strong) token.
        ensure_http_token(&mut http);
        assert_eq!(http.token, first);
    }

    #[test]
    fn secure_token_is_random_and_long() {
        let a = secure_token();
        let b = secure_token();
        assert_eq!(a.len(), 64);
        assert_ne!(a, b, "two CSPRNG tokens must not collide");
    }

    #[test]
    fn legacy_weak_token_is_rotated() {
        // An old time+counter token (two gen_id()s) is short/weak → replaced.
        let mut http = HttpConfig { enabled: true, port: 8787, token: format!("{}{}", gen_id(), gen_id()) };
        assert!(http.token.len() < MIN_TOKEN_LEN);
        ensure_http_token(&mut http);
        assert_eq!(http.token.len(), 64);
    }
}
