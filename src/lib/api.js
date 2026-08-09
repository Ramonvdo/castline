// Thin wrappers over the Tauri IPC bridge. Every backend call goes through here
// so the components stay declarative.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save, open } from "@tauri-apps/plugin-dialog";

// ── Library (folders + items) ──
// Every mutation returns the full LibraryData so the UI re-renders from one source.
export const getLibrary = () => invoke("lib_get_data");
export const libCreateFolder = (name) => invoke("lib_create_folder", { name });
export const libRenameFolder = (id, name) => invoke("lib_rename_folder", { id, name });
export const libDeleteFolder = (id) => invoke("lib_delete_folder", { id });
export const libSetFolderColor = (id, color) => invoke("lib_set_folder_color", { id, color });
export const libSetFolderIcon = (id, icon) => invoke("lib_set_folder_icon", { id, icon });
// Create (empty id) or update a folder's name+icon+color in one atomic save.
export const libUpsertFolder = (id, name, icon, color) =>
  invoke("lib_upsert_folder", { id, name, icon, color });
export const libReorderFolders = (ids) => invoke("lib_reorder_folders", { ids });
export const libSaveItem = (folderId, item) => invoke("lib_save_item", { folderId, item });
export const libDeleteItem = (folderId, itemId) => invoke("lib_delete_item", { folderId, itemId });
export const libMoveItem = (fromFolderId, toFolderId, itemId) =>
  invoke("lib_move_item", { fromFolderId, toFolderId, itemId });
export const libToggleFavorite = (folderId, itemId) =>
  invoke("lib_toggle_favorite", { folderId, itemId });
export const libReorderItems = (folderId, ids) => invoke("lib_reorder_items", { folderId, ids });
export const libRecordUse = (itemId) => invoke("lib_record_use", { itemId });

// ── Profiles (separate store) ──
export const getProfiles = () => invoke("profiles_get_data");
export const profilesSave = (profile) => invoke("profiles_save", { profile });
export const profilesDelete = (id) => invoke("profiles_delete", { id });
export const profilesSetLayout = (layout) => invoke("profiles_set_layout", { layout });
export const profilesSetDescriptions = (descriptions) =>
  invoke("profiles_set_descriptions", { descriptions });
export const profilesSetLocked = (locked) =>
  invoke("profiles_set_locked", { locked });
export const profileFromJson = (jsonText) => invoke("profile_from_json", { jsonText });

// ── Clipboard ──
export const clipCopy = (text) => invoke("clip_copy", { text });
export const clipRead = () => invoke("clip_read");

// ── Settings / outbound connectors ──
export const getSettings = () => invoke("get_settings");
export const setConnectors = (connectors) => invoke("set_connectors", { connectors });
// POST bodyJson to a Make/n8n webhook URL; resolves to { status, body }.
// `label` describes what was sent — it shows up in the Recent-sends log.
export const connectorSend = (url, bodyJson, label = "") =>
  invoke("connector_send", { url, body: bodyJson, label });

// ── Recent sends (payload previews in the Connectors tab) ──
export const getSendHistory = () => invoke("get_send_history");
export const clearSendHistory = () => invoke("clear_send_history");

// ── Inbound HTTP endpoint (Make/n8n HTTP module → profile) ──
export const httpStatus = () => invoke("http_status");
export const setHttpEndpoint = (enabled, port) => invoke("set_http_endpoint", { enabled, port });

// Problems collected while the data stores loaded (drained on read).
export const storageWarnings = () => invoke("storage_warnings");

// ── Castline AI (OpenRouter enrich workflow) ──
// context = user-typed notes / attached file text. Everything else is opt-in
// per run: webSearch (live web), useTone (profile tone → Settings tone),
// useLibrary (templates as reference). tone = the profile's override text.
// itemContext: the ONE template being previewed (the in-modal "AI fill") —
// overrides the library-wide usage context.
export const llmEnrich = (valuesJson, context = "", webSearch = null, tone = "", useTone = false, useLibrary = false, itemContext = "") =>
  invoke("llm_enrich", { values: valuesJson, context, webSearch, tone, useTone, useLibrary, itemContext });
export const setLlmConfig = (apiKey, model, webSearch, tone) =>
  invoke("set_llm_config", { apiKey, model, webSearch, tone });
export const readTextFile = (path) => invoke("read_text_file", { path });

// ── Scheduled jobs ──
export const setSchedules = (schedules) => invoke("set_schedules", { schedules });
export const runScheduleNow = (id) => invoke("run_schedule_now", { id });

// ── Startup & tray ──
export const setAutostart = (enabled) => invoke("set_autostart", { enabled });

// ── Import / export / reveal ──
export const getDataDir = () => invoke("get_data_dir");
export const revealDataDir = () => invoke("reveal_data_dir");
export const exportLibraryTo = (path) => invoke("export_library_to", { path });
export const importLibraryFrom = (path, mode) => invoke("import_library_from", { path, mode });
export const exportProfilesTo = (path) => invoke("export_profiles_to", { path });
export const importProfilesFrom = (path, mode) => invoke("import_profiles_from", { path, mode });
// Write arbitrary text to a path (used by "Export selected → .md").
export const saveTextFile = (path, contents) => invoke("save_text_file", { path, contents });

// ── Blueprints (shareable template files) ──
// build → the JSON text (caller decides: save to a file, or copy to the clipboard).
// parse → preview only, nothing is written. import → the fresh LibraryData.
export const blueprintBuild = (folderId, itemIds) => invoke("blueprint_build", { folderId, itemIds });
export const blueprintParse = (text) => invoke("blueprint_parse", { text });
export const blueprintImport = (folderId, text) => invoke("blueprint_import", { folderId, text });

// ── AI agent (embedded Claude Code terminal) ──
export const aiStatus = () => invoke("ai_status");
export const aiStart = (rows, cols) => invoke("ai_start", { rows, cols });
export const aiInput = (data) => invoke("ai_input", { data });
export const aiResize = (rows, cols) => invoke("ai_resize", { rows, cols });
export const aiStop = () => invoke("ai_stop");
export const refreshAgentContext = () => invoke("refresh_agent_context");
export const setAiConfig = (claudePath, extraArgs) => invoke("set_ai_config", { claudePath, extraArgs });

// ── Live events from the Rust side ──
export const onProfilesChanged = (cb) => listen("profiles-changed", (e) => cb(e.payload));
export const onLibraryChanged = (cb) => listen("library-changed", (e) => cb(e.payload));
export const onScheduleRan = (cb) => listen("schedule-ran", (e) => cb(e.payload));
export const onSendLogged = (cb) => listen("send-logged", (e) => cb(e.payload));
export const onAiOutput = (cb) => listen("ai-output", (e) => cb(e.payload));
export const onAiExit = (cb) => listen("ai-exit", (e) => cb(e.payload));

// ── Native file dialogs ──
const JSON_FILTER = [{ name: "Castline JSON", extensions: ["json"] }];

export async function pickSaveFile(defaultName) {
  const path = await save({ defaultPath: defaultName, filters: JSON_FILTER });
  return typeof path === "string" ? path : null;
}

const DOC_FILTER = [
  { name: "Markdown", extensions: ["md"] },
  { name: "Text", extensions: ["txt"] },
];
export async function pickSaveDoc(defaultName) {
  const path = await save({ defaultPath: defaultName, filters: DOC_FILTER });
  return typeof path === "string" ? path : null;
}

export async function pickOpenFile() {
  const path = await open({ multiple: false, filters: JSON_FILTER });
  return typeof path === "string" ? path : null;
}

// A .txt/.md context file for the AI enrich dialog.
export async function pickContextFile() {
  const path = await open({
    multiple: false,
    filters: [{ name: "Text / Markdown", extensions: ["txt", "md"] }],
  });
  return typeof path === "string" ? path : null;
}

// A target directory (scheduled backups).
export async function pickDirectory() {
  const path = await open({ directory: true, multiple: false });
  return typeof path === "string" ? path : null;
}
