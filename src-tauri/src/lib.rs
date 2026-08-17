//! Castline — Tauri command wiring, managed state, and app setup.

mod blueprint;
mod library;
mod startup;
mod profiles;
mod settings;
mod connectors;
mod receiver;
mod ai;
mod agent;
mod llm;
mod scheduler;
mod history;
mod storage;

use std::path::Path;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

use library::{LibraryData, LibraryState};
use profiles::{ProfilesData, ProfilesState};
use settings::{AppSettings, Connector, SettingsState};

// ─── Library commands ────────────────────────────────────────────────────────
// Each mutation locks the managed state, applies a pure helper from `library`,
// persists, and returns the fresh `LibraryData` so the UI re-renders from one
// source of truth.

fn with_library<F: FnOnce(&mut LibraryData)>(app: &AppHandle, f: F) -> LibraryData {
    let state = app.state::<LibraryState>();
    {
        let mut data = state.data.lock().unwrap();
        f(&mut data);
    }
    state.save();
    let out = state.data.lock().unwrap().clone();
    out
}

#[tauri::command]
fn lib_get_data(app: AppHandle) -> LibraryData {
    app.state::<LibraryState>().data.lock().unwrap().clone()
}

#[tauri::command]
fn lib_create_folder(app: AppHandle, name: String) -> LibraryData {
    with_library(&app, |d| library::create_folder(d, &name))
}

#[tauri::command]
fn lib_rename_folder(app: AppHandle, id: String, name: String) -> LibraryData {
    with_library(&app, |d| library::rename_folder(d, &id, &name))
}

#[tauri::command]
fn lib_delete_folder(app: AppHandle, id: String) -> LibraryData {
    with_library(&app, |d| library::delete_folder(d, &id))
}

#[tauri::command]
fn lib_set_folder_color(app: AppHandle, id: String, color: String) -> LibraryData {
    with_library(&app, |d| library::set_folder_color(d, &id, &color))
}

#[tauri::command]
fn lib_set_folder_icon(app: AppHandle, id: String, icon: String) -> LibraryData {
    with_library(&app, |d| library::set_folder_icon(d, &id, &icon))
}

// Create (empty `id`) or update a folder's name+icon+color in one atomic save —
// used by the folder editor so a just-set icon/color can't be lost to the
// store-watcher reloading an intermediate state.
#[tauri::command]
fn lib_upsert_folder(app: AppHandle, id: String, name: String, icon: String, color: String) -> LibraryData {
    with_library(&app, |d| library::upsert_folder(d, &id, &name, &icon, &color))
}

#[tauri::command]
fn lib_reorder_folders(app: AppHandle, ids: Vec<String>) -> LibraryData {
    with_library(&app, |d| library::reorder_folders(d, &ids))
}

#[tauri::command]
fn lib_save_item(app: AppHandle, folder_id: String, item: library::LibItem) -> LibraryData {
    with_library(&app, |d| library::upsert_item(d, &folder_id, item))
}

#[tauri::command]
fn lib_delete_item(app: AppHandle, folder_id: String, item_id: String) -> LibraryData {
    with_library(&app, |d| library::delete_item(d, &folder_id, &item_id))
}

#[tauri::command]
fn lib_move_item(
    app: AppHandle,
    from_folder_id: String,
    to_folder_id: String,
    item_id: String,
) -> LibraryData {
    with_library(&app, |d| library::move_item(d, &from_folder_id, &to_folder_id, &item_id))
}

#[tauri::command]
fn lib_toggle_favorite(app: AppHandle, folder_id: String, item_id: String) -> LibraryData {
    with_library(&app, |d| library::toggle_favorite(d, &folder_id, &item_id))
}

#[tauri::command]
fn lib_reorder_items(app: AppHandle, folder_id: String, ids: Vec<String>) -> LibraryData {
    with_library(&app, |d| library::reorder_items(d, &folder_id, &ids))
}

/// Bump an item's copy counter (drives the "Most used" sort).
#[tauri::command]
fn lib_record_use(app: AppHandle, item_id: String) -> LibraryData {
    with_library(&app, |d| {
        library::record_use(d, &item_id);
    })
}

// ─── Profile commands ────────────────────────────────────────────────────────

fn with_profiles<F: FnOnce(&mut ProfilesData)>(app: &AppHandle, f: F) -> ProfilesData {
    let state = app.state::<ProfilesState>();
    {
        let mut data = state.data.lock().unwrap();
        f(&mut data);
    }
    state.save();
    let out = state.data.lock().unwrap().clone();
    out
}

#[tauri::command]
fn profiles_get_data(app: AppHandle) -> ProfilesData {
    app.state::<ProfilesState>().data.lock().unwrap().clone()
}

#[tauri::command]
fn profiles_save(app: AppHandle, profile: profiles::Profile) -> ProfilesData {
    with_profiles(&app, |d| profiles::upsert_profile(d, profile))
}

#[tauri::command]
fn profiles_delete(app: AppHandle, id: String) -> ProfilesData {
    with_profiles(&app, |d| profiles::delete_profile(d, &id))
}

/// Replace the global variable layout (splitters + ordering). Presentation-only.
#[tauri::command]
fn profiles_set_layout(app: AppHandle, layout: Vec<profiles::LayoutEntry>) -> ProfilesData {
    with_profiles(&app, |d| profiles::set_layout(d, layout))
}

/// Replace the per-variable descriptions (AI context, edited in Settings).
#[tauri::command]
fn profiles_set_descriptions(
    app: AppHandle,
    descriptions: std::collections::BTreeMap<String, String>,
) -> ProfilesData {
    with_profiles(&app, |d| profiles::set_descriptions(d, descriptions))
}

/// Replace the GLOBAL locked-variable list (locked = empty in every profile,
/// filled on the spot, never written by any enrich path).
#[tauri::command]
fn profiles_set_locked(app: AppHandle, locked: Vec<String>) -> ProfilesData {
    with_profiles(&app, |d| profiles::set_locked(d, locked))
}

/// Paste-importer: build a profile from a raw JSON string (every key passes
/// through as a variable of the same name).
#[tauri::command]
fn profile_from_json(app: AppHandle, json_text: String) -> Result<ProfilesData, String> {
    let payload: serde_json::Value =
        serde_json::from_str(&json_text).map_err(|e| format!("Invalid JSON: {e}"))?;
    let profile = connectors::build_profile_passthrough(&payload)
        .ok_or_else(|| "The JSON must be an object, e.g. { \"first_name\": \"Sam\" }".to_string())?;
    Ok(with_profiles(&app, |d| profiles::upsert_profile(d, profile)))
}

/// Record one outbound send in the Recent-sends log and notify the UI.
pub fn log_send(app: &AppHandle, url: &str, label: &str, body: &str, outcome: &Result<u16, String>) {
    let state = app.state::<history::HistoryState>();
    {
        let mut data = state.data.lock().unwrap();
        history::push(&mut data, history::make_record(url, label, body, outcome));
    }
    state.save();
    let _ = app.emit("send-logged", ());
}

/// POST `body` (JSON) to an outbound connector URL and return its response
/// status + body (the Make/n8n "Webhook response"). The frontend merges/creates.
/// Every call lands in the Recent-sends log (`label` says what was sent).
#[tauri::command]
fn connector_send(
    app: AppHandle,
    url: String,
    body: String,
    label: Option<String>,
) -> Result<connectors::ConnectorResult, String> {
    let result = connectors::connector_send(&url, &body);
    let outcome = match &result {
        Ok(r) => Ok(r.status),
        Err(e) => Err(e.clone()),
    };
    log_send(&app, &url, label.as_deref().unwrap_or(""), &body, &outcome);
    result
}

#[tauri::command]
fn get_send_history(app: AppHandle) -> Vec<history::SendRecord> {
    app.state::<history::HistoryState>().data.lock().unwrap().clone()
}

#[tauri::command]
fn clear_send_history(app: AppHandle) {
    let state = app.state::<history::HistoryState>();
    state.data.lock().unwrap().clear();
    state.save();
}

/// Every library variable paired with its user-written description ("" if none)
/// — the context handed to both the LLM enrich call and the agent's CLAUDE.md.
fn variable_docs(app: &AppHandle) -> Vec<(String, String)> {
    let vars = {
        let state = app.state::<LibraryState>();
        let data = state.data.lock().unwrap();
        library::all_vars(&data)
    };
    let descriptions = {
        let state = app.state::<ProfilesState>();
        let out = state.data.lock().unwrap().descriptions.clone();
        out
    };
    vars.into_iter()
        .map(|v| {
            let d = descriptions.get(&v).cloned().unwrap_or_default();
            (v, d)
        })
        .collect()
}

/// The library templates where variables are used — capped context so the
/// model writes values that read naturally in place (e.g. an icebreaker that
/// fits the sentence around it).
fn usage_context(app: &AppHandle) -> String {
    const PER_ITEM: usize = 700;
    const TOTAL: usize = 5_000;
    let data = {
        let state = app.state::<LibraryState>();
        let out = state.data.lock().unwrap().clone();
        out
    };
    let mut out = String::new();
    for folder in &data.folders {
        for item in &folder.items {
            let mut text = item.text.clone();
            for step in &item.steps {
                if !text.is_empty() {
                    text.push_str("\n\n");
                }
                text.push_str(&step.text);
            }
            if !text.contains("{{") {
                continue; // no variables → no useful context
            }
            let mut snippet: String = text.chars().take(PER_ITEM).collect();
            if snippet.len() < text.len() {
                snippet.push('…');
            }
            let block = format!("### {}\n{}\n\n", item.name, snippet);
            if out.len() + block.len() > TOTAL {
                return out;
            }
            out.push_str(&block);
        }
    }
    out
}

/// "Castline AI" enrich: one OpenRouter call that fills the library's variables
/// for the given profile values. Everything beyond the values is opt-in per
/// run (the enrich dialog's checkboxes): `web_search` (live web), `use_tone`
/// (profile tone → Settings tone; nothing if both empty), `use_library` (the
/// templates where the variables are used). `context` carries user notes / an
/// attached file. Returns a JSON object string (name → value).
#[tauri::command]
fn llm_enrich(
    app: AppHandle,
    values: String,
    context: Option<String>,
    web_search: Option<bool>,
    tone: Option<String>,
    use_tone: Option<bool>,
    use_library: Option<bool>,
    item_context: Option<String>,
) -> Result<String, String> {
    let mut cfg = app.state::<SettingsState>().snapshot().llm;
    if let Some(w) = web_search {
        cfg.web_search = w;
    }
    let vars = variable_docs(&app);
    // The in-preview "AI fill" passes the ONE template being previewed as the
    // usage context; the enrich dialog's checkbox pulls the whole library.
    let usage = match item_context {
        Some(ic) if !ic.trim().is_empty() => ic,
        _ if use_library.unwrap_or(false) => usage_context(&app),
        _ => String::new(),
    };
    let tone = if use_tone.unwrap_or(false) {
        let profile_tone = tone.unwrap_or_default();
        llm::effective_tone(&profile_tone, &cfg.tone).to_string()
    } else {
        String::new()
    };
    let inputs = llm::EnrichInputs {
        vars: &vars,
        context: context.as_deref().unwrap_or(""),
        usage: &usage,
        tone: &tone,
    };
    llm::enrich(&cfg, &values, &inputs)
}

/// Read a small UTF-8 text file (the enrich dialog's .txt/.md attachment).
#[tauri::command]
fn read_text_file(path: String) -> Result<String, String> {
    const MAX: u64 = 1_000_000; // 1 MB is plenty for notes
    let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    if meta.len() > MAX {
        return Err("file is too large (max 1 MB) — trim it down first".into());
    }
    std::fs::read_to_string(&path).map_err(|e| format!("could not read the file as text: {e}"))
}

/// Save the AI-workflow settings (OpenRouter key / model / web research / tone).
#[tauri::command]
fn set_llm_config(
    app: AppHandle,
    api_key: String,
    model: String,
    web_search: bool,
    tone: String,
) -> AppSettings {
    let state = app.state::<SettingsState>();
    {
        let mut s = state.data.lock().unwrap();
        s.llm.api_key = api_key.trim().to_string();
        s.llm.model = model.trim().to_string();
        s.llm.web_search = web_search;
        s.llm.tone = tone.trim().to_string();
    }
    state.save();
    state.snapshot()
}

// ─── Scheduled outbound webhooks ─────────────────────────────────────────────

/// Persist the schedule list (giving each entry a stable id; new entries are
/// anchored at "now" so their first send comes after one full cadence).
#[tauri::command]
fn set_schedules(app: AppHandle, schedules: Vec<settings::Schedule>) -> AppSettings {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let state = app.state::<SettingsState>();
    {
        let mut s = state.data.lock().unwrap();
        let mut list = schedules;
        settings::normalize_schedules(&mut list, now);
        s.schedules = list;
    }
    state.save();
    state.snapshot()
}

// ─── Autostart (launch on login) ─────────────────────────────────────────────

/// Apply the persisted autostart preference to the OS (registry Run key on
/// Windows). Release builds only: a dev exe must never write the entry — it
/// would point login-launch at a debug binary that needs the Vite dev server,
/// and clobber the path a release install registered. Each release launch
/// re-registers, so the entry heals itself after an update or reinstall.
fn apply_autostart(app: &AppHandle, enabled: bool) {
    if cfg!(debug_assertions) {
        return;
    }
    use tauri_plugin_autostart::ManagerExt;
    let mgr = app.autolaunch();
    let _ = if enabled { mgr.enable() } else { mgr.disable() };
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> AppSettings {
    // On a packaged (Store) build Windows owns this, and it may refuse — so the
    // saved value follows what actually happened, never what was asked for.
    let effective = match startup::set(enabled) {
        Some(status) => status.enabled,
        None => {
            apply_autostart(&app, enabled);
            enabled
        }
    };
    let state = app.state::<SettingsState>();
    {
        let mut s = state.data.lock().unwrap();
        s.autostart = effective;
    }
    state.save();
    state.snapshot()
}

/// Real startup state, so Settings can show the truth rather than the last thing
/// that was clicked.
#[tauri::command]
fn autostart_status(app: AppHandle) -> startup::Status {
    let saved = app.state::<SettingsState>().data.lock().unwrap().autostart;
    startup::status(saved)
}

/// Fire one schedule immediately (the "Run now" button).
#[tauri::command]
fn run_schedule_now(app: AppHandle, id: String) -> Result<String, String> {
    scheduler::run_schedule(&app, &id)
}

// ─── Clipboard ───────────────────────────────────────────────────────────────

#[tauri::command]
fn clip_copy(app: AppHandle, text: String) -> bool {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().write_text(text).is_ok()
}

/// Read the clipboard — powers "paste a blueprint someone sent you".
#[tauri::command]
fn clip_read(app: AppHandle) -> Result<String, String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().read_text().map_err(|e| e.to_string())
}

// ─── Settings / connectors ───────────────────────────────────────────────────

#[tauri::command]
fn get_settings(app: AppHandle) -> AppSettings {
    app.state::<SettingsState>().snapshot()
}

/// Persist the list of outbound connectors (giving each a stable id).
#[tauri::command]
fn set_connectors(app: AppHandle, connectors: Vec<Connector>) -> AppSettings {
    let state = app.state::<SettingsState>();
    {
        let mut s = state.data.lock().unwrap();
        let mut cs = connectors;
        settings::normalize_connectors(&mut cs);
        s.connectors = cs;
    }
    state.save();
    state.snapshot()
}

// ─── Inbound HTTP endpoint ───────────────────────────────────────────────────

/// Enable/disable the loopback HTTP endpoint (Make/n8n HTTP module → profile).
/// Enabling mints a bearer token if there isn't one, then (re)starts the server.
/// Errs when the port can't be bound; `enabled` stays persisted so the next
/// launch (or toggle) retries.
#[tauri::command]
fn set_http_endpoint(app: AppHandle, enabled: bool, port: u16) -> Result<AppSettings, String> {
    let state = app.state::<SettingsState>();
    {
        let mut s = state.data.lock().unwrap();
        s.http.enabled = enabled;
        if port != 0 {
            s.http.port = port;
        }
        if enabled {
            settings::ensure_http_token(&mut s.http);
        }
    }
    state.save();
    let snap = state.snapshot();
    app.state::<receiver::HttpController>().apply(&app, snap.http.enabled, snap.http.port)?;
    Ok(snap)
}

/// Current endpoint status for the Connectors UI (and the agent's write path).
#[tauri::command]
fn http_status(app: AppHandle) -> serde_json::Value {
    let s = app.state::<SettingsState>().snapshot().http;
    let active = app.state::<receiver::HttpController>().active_port();
    serde_json::json!({
        "enabled": s.enabled,
        "port": s.port,
        "active": active.is_some(),
        "activePort": active,
        "token": s.token,
        "baseUrl": format!("http://127.0.0.1:{}", s.port),
    })
}

/// Problems collected while the stores loaded (e.g. a corrupt file that was
/// quarantined). Drained on first read so the toast shows once.
#[tauri::command]
fn storage_warnings(app: AppHandle) -> Vec<String> {
    app.state::<storage::StartupWarnings>().0.lock().unwrap().drain(..).collect()
}

// ─── AI agent (embedded Claude Code terminal) ────────────────────────────────

/// Ensure the inbound endpoint is on (with a token) so the agent has a write
/// path, and restart the controller to reflect that.
fn ensure_endpoint_for_agent(app: &AppHandle) {
    let state = app.state::<SettingsState>();
    let (enabled, port) = {
        let mut s = state.data.lock().unwrap();
        s.http.enabled = true;
        settings::ensure_http_token(&mut s.http);
        (s.http.enabled, s.http.port)
    };
    state.save();
    // Idempotent: don't restart a server that's already listening on this port
    // (avoids a bind race with the one started at setup / by the toggle).
    let controller = app.state::<receiver::HttpController>();
    if controller.active_port() != Some(port) {
        if let Err(e) = controller.apply(app, enabled, port) {
            // Agent still works read-only; CLAUDE.md will say the endpoint is off.
            eprintln!("[castline] agent write endpoint unavailable: {e}");
        }
    }
}

/// (Re)generate CLAUDE.md + MEMORY.md in the data dir with the live endpoint.
fn write_agent_context(app: &AppHandle) {
    let http = app.state::<SettingsState>().snapshot().http;
    let active = app.state::<receiver::HttpController>().active_port().is_some();
    let root = settings::app_data_dir();
    let ctx = agent::AgentContext {
        data_dir: root.to_string_lossy().into_owned(),
        base_url: format!("http://127.0.0.1:{}", http.port),
        token: http.token.clone(),
        endpoint_on: http.enabled && active,
        variables: variable_docs(app),
    };
    let _ = agent::write_claude_md(&root, &ctx);
    let _ = agent::ensure_memory_md(&root);
}

/// Status for the Agent tab: is claude installed, where, which workspace, running?
///
/// Also returns enough diagnostics for a user to self-serve when it says "not
/// found" — how it was located, whether this is a packaged (Store/MSIX) build,
/// and every location that was checked. Store installs are the ones that report
/// "not found" on machines where claude is installed, and without this there is
/// nothing for them to send back.
#[tauri::command]
fn ai_status(app: AppHandle) -> serde_json::Value {
    let ai_cfg = app.state::<SettingsState>().snapshot().ai;
    let resolved = ai::resolve_claude(&ai_cfg);
    let probed: Vec<serde_json::Value> = ai::probe_candidates()
        .into_iter()
        .map(|p| {
            serde_json::json!({ "path": p.to_string_lossy(), "found": p.is_file() })
        })
        .collect();
    serde_json::json!({
        "installed": resolved.is_some(),
        "path": resolved.as_ref().map(|r| r.found.clone()),
        "program": resolved.as_ref().map(|r| r.program.clone()),
        "source": resolved.as_ref().map(|r| r.source.as_str()),
        "configured": ai_cfg.claude_path,
        // So a caller changing only the path can write the args back unchanged.
        "extra_args": ai_cfg.extra_args,
        "packaged": startup::is_packaged(),
        "probed": probed,
        "workspace": settings::app_data_dir().to_string_lossy().into_owned(),
        "running": ai::is_running(&app),
    })
}

/// Launch claude in a PTY at the data dir. Ensures the write endpoint is on and
/// regenerates CLAUDE.md first, so the agent starts fully informed + able to write.
#[tauri::command]
fn ai_start(app: AppHandle, rows: u16, cols: u16) -> Result<(), String> {
    let root = settings::app_data_dir();
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    ensure_endpoint_for_agent(&app);
    write_agent_context(&app);
    let ai_cfg = app.state::<SettingsState>().snapshot().ai;
    let mut resolved = ai::resolve_claude(&ai_cfg)
        .ok_or("claude CLI not found — install Claude Code or set a path in Settings".to_string())?;
    resolved.args.extend(ai_cfg.extra_args.clone());
    ai::start(&app, &resolved.program, &resolved.args, &root, rows, cols)
}

#[tauri::command]
fn ai_input(app: AppHandle, data: String) -> Result<(), String> {
    ai::input(&app, &data)
}

#[tauri::command]
fn ai_resize(app: AppHandle, rows: u16, cols: u16) -> Result<(), String> {
    ai::resize(&app, rows, cols)
}

#[tauri::command]
fn ai_stop(app: AppHandle) {
    ai::stop(&app);
}

#[tauri::command]
fn refresh_agent_context(app: AppHandle) -> Result<(), String> {
    write_agent_context(&app);
    Ok(())
}

/// Record that the first-run walkthrough has been offered (or replayed), so the
/// welcome prompt only ever appears once.
#[tauri::command]
fn set_tour_seen(app: AppHandle, seen: bool) -> AppSettings {
    let state = app.state::<SettingsState>();
    {
        let mut s = state.data.lock().unwrap();
        s.tour_seen = seen;
    }
    state.save();
    state.snapshot()
}

#[tauri::command]
fn set_ai_config(app: AppHandle, claude_path: String, extra_args: Vec<String>) -> AppSettings {
    let state = app.state::<SettingsState>();
    {
        let mut s = state.data.lock().unwrap();
        s.ai.claude_path = claude_path;
        s.ai.extra_args = extra_args;
    }
    state.save();
    state.snapshot()
}

// ─── Live reload: reflect external/agent edits to the JSON stores ─────────────

/// Reload a store from disk **only** when the file differs from what we already
/// hold (so our own writes don't echo), then notify the UI.
fn reload_profiles_from_disk(app: &AppHandle) {
    let path = settings::app_data_dir().join("profiles.json");
    let Ok(text) = std::fs::read_to_string(&path) else { return };
    let state = app.state::<ProfilesState>();
    {
        let cur = state.data.lock().unwrap();
        if let Ok(cur_text) = serde_json::to_string_pretty(&*cur) {
            if cur_text.trim() == text.trim() {
                return; // our own write — nothing external changed
            }
        }
    }
    if let Ok(data) = serde_json::from_str::<ProfilesData>(&text) {
        *state.data.lock().unwrap() = data;
        let _ = app.emit("profiles-changed", ());
    }
}

fn reload_library_from_disk(app: &AppHandle) {
    let path = settings::app_data_dir().join("library.json");
    let Ok(text) = std::fs::read_to_string(&path) else { return };
    let state = app.state::<LibraryState>();
    {
        let cur = state.data.lock().unwrap();
        if let Ok(cur_text) = serde_json::to_string_pretty(&*cur) {
            if cur_text.trim() == text.trim() {
                return;
            }
        }
    }
    if let Ok(data) = serde_json::from_str::<LibraryData>(&text) {
        *state.data.lock().unwrap() = data;
        let _ = app.emit("library-changed", ());
    }
}

/// Watch the data dir; live-reload library.json / profiles.json on external edits.
fn spawn_store_watcher(app: AppHandle) {
    use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    std::thread::spawn(move || {
        let dir = settings::app_data_dir();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
            Ok(w) => w,
            Err(_) => return,
        };
        if watcher.watch(&dir, RecursiveMode::NonRecursive).is_err() {
            return;
        }
        for res in rx {
            let Ok(event) = res else { continue };
            if !matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                continue;
            }
            for path in &event.paths {
                match path.file_name().and_then(|n| n.to_str()) {
                    Some("profiles.json") => reload_profiles_from_disk(&app),
                    Some("library.json") => reload_library_from_disk(&app),
                    _ => {}
                }
            }
        }
    });
}

/// Give the (undecorated) window a fixed border colour instead of the system
/// accent — Castline's muted blue-grey #9AABB0. Windows 11 only; no-op elsewhere.
#[cfg(windows)]
fn set_window_border(window: &tauri::WebviewWindow) {
    use std::ffi::c_void;
    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(hwnd: isize, attr: u32, pv: *const c_void, cb: u32) -> i32;
    }
    const DWMWA_BORDER_COLOR: u32 = 34;
    if let Ok(hwnd) = window.hwnd() {
        // COLORREF is 0x00BBGGRR — #313D48 → 0x00483D31.
        let color: u32 = 0x00483D31;
        unsafe {
            DwmSetWindowAttribute(
                hwnd.0 as isize,
                DWMWA_BORDER_COLOR,
                &color as *const u32 as *const c_void,
                4,
            );
        }
    }
}

#[cfg(not(windows))]
fn set_window_border(_window: &tauri::WebviewWindow) {}

// ─── Import / export / reveal ────────────────────────────────────────────────

#[tauri::command]
fn get_data_dir() -> String {
    settings::app_data_dir().to_string_lossy().into_owned()
}

#[tauri::command]
fn reveal_data_dir(app: AppHandle) -> Result<(), String> {
    let dir = settings::app_data_dir();
    // Reveal library.json inside the data dir (a concrete item the OS can select).
    let target = dir.join("library.json");
    let to_open = if target.exists() { target } else { dir };
    if !to_open.exists() {
        return Err("data folder does not exist yet".into());
    }
    app.opener().reveal_item_in_dir(&to_open).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_library_to(app: AppHandle, path: String) -> Result<(), String> {
    let data = app.state::<LibraryState>().data.lock().unwrap().clone();
    let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_library_from(app: AppHandle, path: String, mode: String) -> Result<LibraryData, String> {
    if !Path::new(&path).exists() {
        return Err("file no longer exists".into());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let imported: LibraryData =
        serde_json::from_str(&text).map_err(|e| format!("Not a valid Castline library file: {e}"))?;
    if mode == "replace" {
        app.state::<LibraryState>().replace(imported);
        Ok(app.state::<LibraryState>().data.lock().unwrap().clone())
    } else {
        Ok(with_library(&app, |d| library::merge_import(d, imported)))
    }
}

/// Write arbitrary UTF-8 text to `path` (powers "Export selected → .md").
#[tauri::command]
fn save_text_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents).map_err(|e| e.to_string())
}

// ─── Blueprints (shareable template files) ───────────────────────────────────

/// Serialize the given items as a blueprint. `folder_id` only decides whether
/// folder presentation rides along — the items are looked up library-wide, so a
/// multi-select spanning folders exports fine.
#[tauri::command]
fn blueprint_build(
    app: AppHandle,
    folder_id: Option<String>,
    item_ids: Vec<String>,
) -> Result<String, String> {
    let data = app.state::<LibraryState>().data.lock().unwrap().clone();
    // Resolve in the order asked for, so a picked selection exports in pick order.
    let items: Vec<library::LibItem> = item_ids
        .iter()
        .filter_map(|id| {
            data.folders
                .iter()
                .flat_map(|f| f.items.iter())
                .find(|i| &i.id == id)
                .cloned()
        })
        .collect();
    if items.is_empty() {
        return Err("Nothing to export.".into());
    }
    let folder = folder_id
        .as_deref()
        .and_then(|fid| data.folders.iter().find(|f| f.id == fid));
    let bp = blueprint::from_items(&items, folder, env!("CARGO_PKG_VERSION"));
    serde_json::to_string_pretty(&bp).map_err(|e| e.to_string())
}

/// Parse without importing — powers the preview shown before anything lands.
#[tauri::command]
fn blueprint_parse(text: String) -> Result<blueprint::Blueprint, String> {
    blueprint::parse(&text)
}

/// Import a blueprint into `folder_id`, or into a folder created from the
/// blueprint's own folder metadata when none is given.
#[tauri::command]
fn blueprint_import(
    app: AppHandle,
    folder_id: Option<String>,
    text: String,
) -> Result<LibraryData, String> {
    let bp = blueprint::parse(&text)?;
    let items = blueprint::to_lib_items(&bp);

    if let Some(id) = folder_id.as_deref() {
        // upsert_item silently no-ops on an unknown folder, which would look like
        // a successful import that quietly dropped everything. Fail loudly instead.
        let known = app
            .state::<LibraryState>()
            .data
            .lock()
            .unwrap()
            .folders
            .iter()
            .any(|f| f.id == id);
        if !known {
            return Err("That folder no longer exists.".into());
        }
    }

    // Create-the-folder and insert-the-items happen in ONE save: two saves in a
    // row race the store file-watcher, which can re-apply the snapshot it read
    // between them and silently undo the second half.
    Ok(with_library(&app, |d| {
        let target = match folder_id.clone() {
            Some(id) => id,
            None => {
                let name = bp
                    .folder
                    .as_ref()
                    .map(|f| f.name.trim().to_string())
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| "Imported".to_string());
                library::create_folder(d, &name);
                match d.folders.last_mut() {
                    Some(f) => {
                        if let Some(meta) = bp.folder.as_ref() {
                            f.icon = meta.icon.clone();
                            f.color = meta.color.clone();
                        }
                        f.id.clone()
                    }
                    None => return,
                }
            }
        };
        for item in items {
            library::upsert_item(d, &target, item);
        }
    }))
}

#[tauri::command]
fn export_profiles_to(app: AppHandle, path: String) -> Result<(), String> {
    let data = app.state::<ProfilesState>().data.lock().unwrap().clone();
    let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_profiles_from(app: AppHandle, path: String, mode: String) -> Result<ProfilesData, String> {
    if !Path::new(&path).exists() {
        return Err("file no longer exists".into());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let imported: ProfilesData =
        serde_json::from_str(&text).map_err(|e| format!("Not a valid Castline profiles file: {e}"))?;
    if mode == "replace" {
        app.state::<ProfilesState>().replace(imported);
        Ok(app.state::<ProfilesState>().data.lock().unwrap().clone())
    } else {
        Ok(with_profiles(&app, |d| profiles::merge_import(d, imported)))
    }
}

// ─── App entry ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = settings::app_data_dir();

    // Load the stores before the builder so problems (quarantined corrupt
    // files) can be collected and surfaced once the webview is up.
    let mut warnings = Vec::new();
    let library = LibraryState::load(data_dir.join("library.json"), &mut warnings);
    let profiles = ProfilesState::load(data_dir.join("profiles.json"), &mut warnings);
    let app_settings = SettingsState::load(&mut warnings);
    let history = history::HistoryState::load(data_dir.join("history.json"), &mut warnings);

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(library)
        .manage(profiles)
        .manage(app_settings)
        .manage(receiver::HttpController::default())
        .manage(ai::AiState::default())
        .manage(history)
        .manage(storage::StartupWarnings(std::sync::Mutex::new(warnings)))
        .setup(|app| {
            let handle = app.handle();
            // Start the inbound HTTP endpoint if it was left enabled.
            let http = handle.state::<SettingsState>().snapshot().http;
            if http.enabled {
                if let Err(e) = handle.state::<receiver::HttpController>().apply(handle, true, http.port) {
                    let warnings = handle.state::<storage::StartupWarnings>();
                    warnings.0.lock().unwrap().push(format!("HTTP endpoint couldn't start: {e}"));
                }
            }
            // Live-reload the JSON stores when they change on disk.
            spawn_store_watcher(handle.clone());
            // Scheduled jobs (missed runs re-anchor; first tick after ~90s).
            scheduler::spawn(handle.clone());
            // Fixed window border colour instead of the system accent.
            if let Some(win) = handle.get_webview_window("main") {
                set_window_border(&win);
            }
            // Launch-on-login: sync the OS Run entry to the persisted setting.
            let autostart = handle.state::<SettingsState>().snapshot().autostart;
            apply_autostart(handle, autostart);
            // When the OS launched us at login, stay in the tray instead of
            // popping a window over whatever the user is doing.
            if std::env::args().any(|a| a == "--autostart") {
                if let Some(win) = handle.get_webview_window("main") {
                    let _ = win.hide();
                }
            }

            // System tray: Castline keeps running (scheduler, endpoint, agent)
            // when the window is closed; reopen or quit from here.
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

                let show_main = |app: &AppHandle| {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.unminimize();
                        let _ = w.set_focus();
                    }
                };
                let open_i = MenuItem::with_id(app, "open", "Open Castline", true, None::<&str>)?;
                let quit_i = MenuItem::with_id(app, "quit", "Quit Castline", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&open_i, &quit_i])?;
                let mut tray = TrayIconBuilder::with_id("castline-tray")
                    .tooltip("Castline")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(move |app, event| match event.id.as_ref() {
                        "open" => show_main(app),
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.unminimize();
                                let _ = w.set_focus();
                            }
                        }
                    });
                if let Some(icon) = app.default_window_icon() {
                    tray = tray.icon(icon.clone());
                }
                tray.build(app)?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                // Close = hide to the tray; the app (scheduler, endpoint,
                // agent) keeps running. Quit lives in the tray menu.
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                // Tear the PTY down with the window so no claude child lingers.
                tauri::WindowEvent::Destroyed => {
                    ai::stop(window.app_handle());
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            lib_get_data,
            lib_create_folder,
            lib_rename_folder,
            lib_delete_folder,
            lib_set_folder_color,
            lib_set_folder_icon,
            lib_upsert_folder,
            lib_reorder_folders,
            lib_save_item,
            lib_delete_item,
            lib_move_item,
            lib_toggle_favorite,
            lib_reorder_items,
            lib_record_use,
            profiles_get_data,
            profiles_save,
            profiles_delete,
            profiles_set_layout,
            profiles_set_descriptions,
            profiles_set_locked,
            profile_from_json,
            connector_send,
            get_send_history,
            clear_send_history,
            llm_enrich,
            set_llm_config,
            set_schedules,
            run_schedule_now,
            set_autostart,
            autostart_status,
            read_text_file,
            clip_copy,
            clip_read,
            blueprint_build,
            blueprint_parse,
            blueprint_import,
            get_settings,
            set_connectors,
            set_http_endpoint,
            http_status,
            storage_warnings,
            ai_status,
            ai_start,
            ai_input,
            ai_resize,
            ai_stop,
            refresh_agent_context,
            set_ai_config,
            set_tour_seen,
            get_data_dir,
            reveal_data_dir,
            export_library_to,
            import_library_from,
            export_profiles_to,
            import_profiles_from,
            save_text_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Castline");
}
