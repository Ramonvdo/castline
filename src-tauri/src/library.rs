//! The Castline library — a local store of reusable prompts / templates / notes / SOPs.
//!
//! Self-contained JSON store (`<data_dir>/Castline/library.json`). Folders hold
//! items; each item is either a single `template` (one block of text) or a
//! multi-step `sop` (an ordered chain of copy-pasteable prompts). Variable
//! *profiles* live in a **separate** file/store (`profiles.rs`), so a prompt
//! library and the people/values you fill it with can be backed up and shared
//! independently.
//!
//! Variable extraction/substitution (`{{name}}` tokens) lives in the frontend;
//! this module only persists data. All mutation helpers are pure functions over
//! `LibraryData` so they can be unit-tested without Tauri.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// ─── Data models ────────────────────────────────────────────────────────────

/// One step of a multi-step SOP (an ordered, copy-pasteable prompt).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopStep {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub text: String,
}

/// A library entry: a single template, or an ordered SOP of steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibItem {
    pub id: String,
    pub name: String,
    /// "template" | "sop".
    #[serde(default = "default_kind")]
    pub kind: String,
    /// Free-form label ("prompt" | "note" | "email" | "snippet" | …) — purely a
    /// cosmetic icon + a filter, so there's no hard prompt/template/note divide.
    #[serde(rename = "type", default)]
    pub item_type: String,
    /// Used when `kind == "template"`.
    #[serde(default)]
    pub text: String,
    /// Used when `kind == "sop"`.
    #[serde(default)]
    pub steps: Vec<SopStep>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

/// A folder grouping items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibFolder {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub color: String,
    /// An emoji shown next to the folder for fast visual scanning.
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub items: Vec<LibItem>,
}

/// The whole library (one JSON file). Profiles are stored separately.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryData {
    #[serde(default)]
    pub folders: Vec<LibFolder>,
}

fn default_kind() -> String {
    "template".into()
}

impl Default for LibraryData {
    fn default() -> Self {
        // Seed with a couple of examples so the app demonstrates the feature.
        LibraryData {
            folders: vec![LibFolder {
                id: gen_id(),
                name: "General".into(),
                color: "#4f8cff".into(),
                icon: String::new(),
                items: vec![
                    LibItem {
                        id: gen_id(),
                        name: "Cold outreach".into(),
                        kind: "template".into(),
                        item_type: "email".into(),
                        text: "Hi {{firstName}},\n\nI came across {{company}} and loved what \
                               you're doing. I help teams like {{company}} {{outcome}} — would \
                               {{firstName}} be open to a quick chat this week?\n\nBest,\n{{myName}}"
                            .into(),
                        steps: vec![],
                        tags: vec!["copywriting".into(), "email".into()],
                        favorite: false,
                        created_at: now_iso(),
                        updated_at: now_iso(),
                    },
                    LibItem {
                        id: gen_id(),
                        name: "Blog post SOP".into(),
                        kind: "sop".into(),
                        item_type: "prompt".into(),
                        text: String::new(),
                        steps: vec![
                            SopStep {
                                id: gen_id(),
                                title: "1 · Outline".into(),
                                text: "Act as an expert content strategist. Create a detailed \
                                       outline for a blog post about {{topic}} aimed at \
                                       {{audience}}."
                                    .into(),
                            },
                            SopStep {
                                id: gen_id(),
                                title: "2 · Draft".into(),
                                text: "Using the outline above, write a full first draft about \
                                       {{topic}} in a {{tone}} tone for {{audience}}."
                                    .into(),
                            },
                            SopStep {
                                id: gen_id(),
                                title: "3 · Polish".into(),
                                text: "Edit the draft for clarity and flow. Keep the {{tone}} \
                                       tone and tighten it for {{audience}}."
                                    .into(),
                            },
                        ],
                        tags: vec!["content".into(), "workflow".into()],
                        favorite: false,
                        created_at: now_iso(),
                        updated_at: now_iso(),
                    },
                ],
            }],
        }
    }
}

// ─── Id / time helpers ───────────────────────────────────────────────────────

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Short, collision-resistant id from the clock + a process-wide counter.
pub fn gen_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:x}{:x}", nanos, n)
}

/// `YYYY-MM-DDTHH:MM:SS` in local time.
pub fn now_iso() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

// ─── Managed state (Tauri) ──────────────────────────────────────────────────

/// Tauri-managed live library + the JSON path it persists to.
pub struct LibraryState {
    pub data: Mutex<LibraryData>,
    pub path: PathBuf,
}

impl LibraryState {
    /// Load from `path`, falling back to seeded defaults (and writing them) when
    /// the file is missing or unreadable.
    pub fn load(path: PathBuf) -> Self {
        let data = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => {
                let d = LibraryData::default();
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                if let Ok(json) = serde_json::to_string_pretty(&d) {
                    let _ = std::fs::write(&path, json);
                }
                d
            }
        };
        LibraryState { data: Mutex::new(data), path }
    }

    /// Persist the current data to disk (best-effort, pretty JSON).
    pub fn save(&self) {
        if let Ok(data) = self.data.lock() {
            if let Some(dir) = self.path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_string_pretty(&*data) {
                let _ = std::fs::write(&self.path, json);
            }
        }
    }

    /// Replace the whole library (used by Import → replace).
    pub fn replace(&self, new_data: LibraryData) {
        if let Ok(mut data) = self.data.lock() {
            *data = new_data;
        }
        self.save();
    }
}

// ─── Pure mutation helpers (unit-tested) ─────────────────────────────────────

/// Stamp ids + timestamps on an incoming item from the frontend. A blank `id`
/// means "new"; existing ids are preserved so updates stay in place.
fn normalize_item(mut item: LibItem) -> LibItem {
    if item.id.trim().is_empty() {
        item.id = gen_id();
    }
    if item.created_at.trim().is_empty() {
        item.created_at = now_iso();
    }
    item.updated_at = now_iso();
    if item.kind != "sop" {
        item.kind = "template".into();
    }
    for step in &mut item.steps {
        if step.id.trim().is_empty() {
            step.id = gen_id();
        }
    }
    item
}

pub fn create_folder(data: &mut LibraryData, name: &str) {
    data.folders.push(LibFolder {
        id: gen_id(),
        name: name.trim().to_string(),
        color: String::new(),
        icon: String::new(),
        items: vec![],
    });
}

pub fn rename_folder(data: &mut LibraryData, id: &str, name: &str) {
    if let Some(f) = data.folders.iter_mut().find(|f| f.id == id) {
        f.name = name.trim().to_string();
    }
}

pub fn delete_folder(data: &mut LibraryData, id: &str) {
    data.folders.retain(|f| f.id != id);
}

pub fn set_folder_color(data: &mut LibraryData, id: &str, color: &str) {
    if let Some(f) = data.folders.iter_mut().find(|f| f.id == id) {
        f.color = color.to_string();
    }
}

pub fn set_folder_icon(data: &mut LibraryData, id: &str, icon: &str) {
    if let Some(f) = data.folders.iter_mut().find(|f| f.id == id) {
        f.icon = icon.to_string();
    }
}

pub fn reorder_folders(data: &mut LibraryData, ids: &[String]) {
    let mut reordered: Vec<LibFolder> = Vec::with_capacity(data.folders.len());
    for id in ids {
        if let Some(pos) = data.folders.iter().position(|f| &f.id == id) {
            reordered.push(data.folders.remove(pos));
        }
    }
    reordered.append(&mut data.folders); // keep any not named in `ids`
    data.folders = reordered;
}

/// Insert (blank id) or replace (matching id) an item in `folder_id`.
pub fn upsert_item(data: &mut LibraryData, folder_id: &str, item: LibItem) {
    let item = normalize_item(item);
    if let Some(folder) = data.folders.iter_mut().find(|f| f.id == folder_id) {
        if let Some(existing) = folder.items.iter_mut().find(|i| i.id == item.id) {
            *existing = item;
        } else {
            folder.items.push(item);
        }
    }
}

pub fn delete_item(data: &mut LibraryData, folder_id: &str, item_id: &str) {
    if let Some(folder) = data.folders.iter_mut().find(|f| f.id == folder_id) {
        folder.items.retain(|i| i.id != item_id);
    }
}

pub fn move_item(data: &mut LibraryData, from: &str, to: &str, item_id: &str) {
    let taken = data
        .folders
        .iter_mut()
        .find(|f| f.id == from)
        .and_then(|f| f.items.iter().position(|i| i.id == item_id).map(|p| f.items.remove(p)));
    if let Some(item) = taken {
        if let Some(dest) = data.folders.iter_mut().find(|f| f.id == to) {
            dest.items.push(item);
        }
    }
}

pub fn toggle_favorite(data: &mut LibraryData, folder_id: &str, item_id: &str) {
    if let Some(folder) = data.folders.iter_mut().find(|f| f.id == folder_id) {
        if let Some(item) = folder.items.iter_mut().find(|i| i.id == item_id) {
            item.favorite = !item.favorite;
        }
    }
}

/// Merge an imported library into the current one: append every folder with a
/// fresh id (and fresh item/step ids) so nothing collides or overwrites.
pub fn merge_import(data: &mut LibraryData, imported: LibraryData) {
    for mut folder in imported.folders {
        folder.id = gen_id();
        for item in &mut folder.items {
            item.id = gen_id();
            for step in &mut item.steps {
                step.id = gen_id();
            }
        }
        data.folders.push(folder);
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn blank_item(name: &str) -> LibItem {
        LibItem {
            id: String::new(),
            name: name.into(),
            kind: "template".into(),
            item_type: String::new(),
            text: "hello {{x}}".into(),
            steps: vec![],
            tags: vec![],
            favorite: false,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn default_is_seeded() {
        let d = LibraryData::default();
        assert_eq!(d.folders.len(), 1);
        assert!(d.folders[0].items.len() >= 2);
        assert!(d.folders[0].items.iter().any(|i| i.kind == "sop"));
    }

    #[test]
    fn upsert_assigns_then_preserves_id() {
        let mut d = LibraryData { folders: vec![] };
        create_folder(&mut d, "F");
        let fid = d.folders[0].id.clone();

        upsert_item(&mut d, &fid, blank_item("A"));
        assert_eq!(d.folders[0].items.len(), 1);
        let iid = d.folders[0].items[0].id.clone();
        assert!(!iid.is_empty());

        let mut edit = d.folders[0].items[0].clone();
        edit.text = "world {{y}}".into();
        upsert_item(&mut d, &fid, edit);
        assert_eq!(d.folders[0].items.len(), 1);
        assert_eq!(d.folders[0].items[0].text, "world {{y}}");
        assert_eq!(d.folders[0].items[0].id, iid);
    }

    #[test]
    fn delete_and_move_item() {
        let mut d = LibraryData { folders: vec![] };
        create_folder(&mut d, "A");
        create_folder(&mut d, "B");
        let (a, b) = (d.folders[0].id.clone(), d.folders[1].id.clone());
        upsert_item(&mut d, &a, blank_item("x"));
        let iid = d.folders[0].items[0].id.clone();

        move_item(&mut d, &a, &b, &iid);
        assert!(d.folders[0].items.is_empty());
        assert_eq!(d.folders[1].items.len(), 1);

        delete_item(&mut d, &b, &iid);
        assert!(d.folders[1].items.is_empty());
    }

    #[test]
    fn reorder_keeps_unlisted() {
        let mut d = LibraryData { folders: vec![] };
        create_folder(&mut d, "A");
        create_folder(&mut d, "B");
        create_folder(&mut d, "C");
        let ids: Vec<String> = d.folders.iter().map(|f| f.id.clone()).collect();
        reorder_folders(&mut d, &[ids[2].clone(), ids[0].clone()]);
        assert_eq!(d.folders[0].name, "C");
        assert_eq!(d.folders[1].name, "A");
        assert_eq!(d.folders[2].name, "B");
    }

    #[test]
    fn merge_import_reids_and_appends() {
        let mut d = LibraryData { folders: vec![] };
        create_folder(&mut d, "Existing");
        let existing_id = d.folders[0].id.clone();

        let mut incoming = LibraryData { folders: vec![] };
        create_folder(&mut incoming, "Imported");
        let incoming_id = incoming.folders[0].id.clone();
        upsert_item(&mut incoming, &incoming_id, blank_item("i1"));

        merge_import(&mut d, incoming);
        assert_eq!(d.folders.len(), 2);
        // Imported folder got a fresh id (no collision with its origin or ours).
        assert!(d.folders.iter().all(|f| f.id != incoming_id));
        assert!(d.folders.iter().any(|f| f.id == existing_id));
        assert!(d.folders.iter().any(|f| f.name == "Imported"));
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.json");
        let state = LibraryState::load(path.clone());
        {
            let mut data = state.data.lock().unwrap();
            create_folder(&mut data, "Roundtrip");
        }
        state.save();

        let reloaded = LibraryState::load(path);
        let data = reloaded.data.lock().unwrap();
        assert!(data.folders.iter().any(|f| f.name == "Roundtrip"));
    }
}
