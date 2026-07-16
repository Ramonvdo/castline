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
export const profileFromJson = (jsonText) => invoke("profile_from_json", { jsonText });

// ── Clipboard ──
export const clipCopy = (text) => invoke("clip_copy", { text });

// ── Settings / outbound connectors ──
export const getSettings = () => invoke("get_settings");
export const setConnectors = (connectors) => invoke("set_connectors", { connectors });
// POST bodyJson to a Make/n8n webhook URL; resolves to { status, body }.
export const connectorSend = (url, bodyJson) => invoke("connector_send", { url, body: bodyJson });

// ── Inbound HTTP endpoint (Make/n8n HTTP module → profile) ──
export const httpStatus = () => invoke("http_status");
export const setHttpEndpoint = (enabled, port) => invoke("set_http_endpoint", { enabled, port });

// ── Castline AI (OpenRouter enrich workflow) ──
export const llmEnrich = (valuesJson) => invoke("llm_enrich", { values: valuesJson });
export const setLlmConfig = (apiKey, model, webSearch) =>
  invoke("set_llm_config", { apiKey, model, webSearch });

// ── Scheduled outbound webhooks ──
export const setSchedules = (schedules) => invoke("set_schedules", { schedules });
export const runScheduleNow = (id) => invoke("run_schedule_now", { id });

// ── Import / export / reveal ──
export const getDataDir = () => invoke("get_data_dir");
export const revealDataDir = () => invoke("reveal_data_dir");
export const exportLibraryTo = (path) => invoke("export_library_to", { path });
export const importLibraryFrom = (path, mode) => invoke("import_library_from", { path, mode });
export const exportProfilesTo = (path) => invoke("export_profiles_to", { path });
export const importProfilesFrom = (path, mode) => invoke("import_profiles_from", { path, mode });
// Write arbitrary text to a path (used by "Export selected → .md").
export const saveTextFile = (path, contents) => invoke("save_text_file", { path, contents });

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
