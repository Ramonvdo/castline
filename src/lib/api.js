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

// ── Profiles (separate store) ──
export const getProfiles = () => invoke("profiles_get_data");
export const profilesSave = (profile) => invoke("profiles_save", { profile });
export const profilesDelete = (id) => invoke("profiles_delete", { id });
export const profilesSetLayout = (layout) => invoke("profiles_set_layout", { layout });
export const profileFromJson = (jsonText) => invoke("profile_from_json", { jsonText });

// ── Clipboard ──
export const clipCopy = (text) => invoke("clip_copy", { text });

// ── Settings / webhooks ──
export const getSettings = () => invoke("get_settings");
export const setReceiver = (config) => invoke("set_receiver", { config });
export const webhookStatus = () => invoke("webhook_status");
export const webhookPreview = (webhook, jsonText) => invoke("webhook_preview", { webhook, jsonText });

// ── Import / export / reveal ──
export const getDataDir = () => invoke("get_data_dir");
export const revealDataDir = () => invoke("reveal_data_dir");
export const exportLibraryTo = (path) => invoke("export_library_to", { path });
export const importLibraryFrom = (path, mode) => invoke("import_library_from", { path, mode });
export const exportProfilesTo = (path) => invoke("export_profiles_to", { path });
export const importProfilesFrom = (path, mode) => invoke("import_profiles_from", { path, mode });
// Write arbitrary text to a path (used by "Export selected → .md").
export const saveTextFile = (path, contents) => invoke("save_text_file", { path, contents });

// ── Live events from the Rust side ──
export const onProfilesChanged = (cb) => listen("profiles-changed", (e) => cb(e.payload));

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
