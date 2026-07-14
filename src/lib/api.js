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

// ── Settings / appearance / webhook ──
export const getSettings = () => invoke("get_settings");
export const setAccent = (accent) => invoke("set_accent", { accent });
export const setWebhookConfig = (config) => invoke("set_webhook_config", { config });
export const webhookStatus = () => invoke("webhook_status");

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

// ── Theming ──
// Blend a hex accent toward a base hex by `amt` (0..1), returning an rgb() string.
// Pure JS so it never depends on CSS color-mix.
function blendHex(accent, base, amt) {
  const toRgb = (h) => {
    let s = String(h || "").replace("#", "").trim();
    if (s.length === 3) s = s.split("").map((c) => c + c).join("");
    if (!/^[0-9a-fA-F]{6}$/.test(s)) return null;
    const n = parseInt(s, 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  };
  const a = toRgb(accent);
  const b = toRgb(base);
  if (!a || !b) return base;
  const c = (i) => Math.round(a[i] * amt + b[i] * (1 - amt));
  return `rgb(${c(0)}, ${c(1)}, ${c(2)})`;
}

// Apply an accent to the document's CSS variables. Surfaces are tinted toward the
// accent so the whole (dark) UI shifts hue with it, computed in JS.
export function applyAccent(accent) {
  const root = document.documentElement;
  const a = accent || "#5f9cf2";
  root.style.setProperty("--accent", a);
  // Navy bases keep the whole UI monochrome-blue; a light accent blend adds the
  // faint metallic tint without ever going neutral-grey.
  root.style.setProperty("--bg", blendHex(a, "#0a0f1a", 0.05));
  root.style.setProperty("--surface", blendHex(a, "#0f1826", 0.06));
  root.style.setProperty("--elevated", blendHex(a, "#16223a", 0.08));
  root.style.setProperty("--border", blendHex(a, "#26314c", 0.16));
}
