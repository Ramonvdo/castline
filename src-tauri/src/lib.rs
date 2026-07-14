//! Castline — Tauri command wiring, managed state, and app setup.

mod library;
mod profiles;
mod settings;
mod connectors;

use std::path::Path;

use tauri::{AppHandle, Manager};
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

/// POST `body` (JSON) to an outbound connector URL and return its response
/// status + body (the Make/n8n "Webhook response"). The frontend merges/creates.
#[tauri::command]
fn connector_send(url: String, body: String) -> Result<connectors::ConnectorResult, String> {
    connectors::connector_send(&url, &body)
}

// ─── Clipboard ───────────────────────────────────────────────────────────────

#[tauri::command]
fn clip_copy(app: AppHandle, text: String) -> bool {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().write_text(text).is_ok()
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

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(LibraryState::load(data_dir.join("library.json")))
        .manage(ProfilesState::load(data_dir.join("profiles.json")))
        .manage(SettingsState::load())
        .invoke_handler(tauri::generate_handler![
            lib_get_data,
            lib_create_folder,
            lib_rename_folder,
            lib_delete_folder,
            lib_set_folder_color,
            lib_set_folder_icon,
            lib_reorder_folders,
            lib_save_item,
            lib_delete_item,
            lib_move_item,
            lib_toggle_favorite,
            lib_reorder_items,
            profiles_get_data,
            profiles_save,
            profiles_delete,
            profiles_set_layout,
            profile_from_json,
            connector_send,
            clip_copy,
            get_settings,
            set_connectors,
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
