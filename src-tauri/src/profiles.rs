//! Variable *profiles* — a store kept **separate** from the library
//! (`<data_dir>/Castline/profiles.json`), so the people/values you fill
//! templates with can be backed up, shared and imported independently of the
//! prompts themselves.
//!
//! A `Profile` is a named set of `{{variable}} -> value` pairs used by the
//! frontend's "Fill & copy" flow, and the target that incoming webhooks
//! (`webhook.rs`) create automatically.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::library::gen_id;

/// A reusable, named set of variable values for "Fill & copy".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub values: BTreeMap<String, String>,
    /// "manual" | "webhook" | "import" — where the profile came from.
    #[serde(default)]
    pub source: String,
    /// Optional tone-of-voice override for AI-generated values (beats the
    /// global tone in Settings when non-empty).
    #[serde(default)]
    pub tone: String,
}

/// One entry in the **global variable layout** — either a `splitter` (a labelled
/// section header) or a `var` (a placement of a variable by name). This is
/// presentation-only metadata: it never affects any `Profile.values`, so incoming
/// webhooks / n8n / Make mappings (which key on variable *names*) keep working no
/// matter how the layout is reorganised.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutEntry {
    #[serde(rename = "type")]
    pub kind: String,
    /// Used when `kind == "splitter"`.
    #[serde(default)]
    pub label: String,
    /// Used when `kind == "var"`.
    #[serde(default)]
    pub name: String,
}

/// The whole profiles store (one JSON file).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfilesData {
    #[serde(default)]
    pub profiles: Vec<Profile>,
    /// Global grouping of variables shared by every profile + the Fill panel.
    #[serde(default)]
    pub layout: Vec<LayoutEntry>,
    /// Per-variable descriptions — context for the AI enrich workflow and the
    /// agent's CLAUDE.md (what each `{{variable}}` should contain, with examples).
    #[serde(default)]
    pub descriptions: BTreeMap<String, String>,
}

/// Tauri-managed live profiles + the JSON path it persists to.
pub struct ProfilesState {
    pub data: Mutex<ProfilesData>,
    pub path: PathBuf,
}

impl ProfilesState {
    pub fn load(path: PathBuf) -> Self {
        let data = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => {
                let d = ProfilesData::default();
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                if let Ok(json) = serde_json::to_string_pretty(&d) {
                    let _ = std::fs::write(&path, json);
                }
                d
            }
        };
        ProfilesState { data: Mutex::new(data), path }
    }

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

    pub fn replace(&self, new_data: ProfilesData) {
        if let Ok(mut data) = self.data.lock() {
            *data = new_data;
        }
        self.save();
    }
}

// ─── Pure mutation helpers (unit-tested) ─────────────────────────────────────

/// Insert (blank id) or replace (matching id) a profile.
pub fn upsert_profile(data: &mut ProfilesData, mut profile: Profile) {
    if profile.id.trim().is_empty() {
        profile.id = gen_id();
    }
    if profile.source.trim().is_empty() {
        profile.source = "manual".into();
    }
    if let Some(existing) = data.profiles.iter_mut().find(|p| p.id == profile.id) {
        *existing = profile;
    } else {
        data.profiles.push(profile);
    }
}

pub fn delete_profile(data: &mut ProfilesData, id: &str) {
    data.profiles.retain(|p| p.id != id);
}

/// Merge `incoming.values` into an **existing** profile, matched by name
/// (case-insensitive) or by a shared `email` value. Returns the matched
/// profile's name, or `None` when nothing matches. Used by the inbound
/// `/api/update-profile` endpoint (enrichment).
pub fn enrich_existing(data: &mut ProfilesData, incoming: &Profile) -> Option<String> {
    let want_name = incoming.name.trim().to_ascii_lowercase();
    let want_email =
        incoming.values.get("email").map(|e| e.trim().to_ascii_lowercase()).filter(|e| !e.is_empty());

    let target = data.profiles.iter_mut().find(|p| {
        (!want_name.is_empty() && p.name.trim().to_ascii_lowercase() == want_name)
            || match (&want_email, p.values.get("email")) {
                (Some(w), Some(have)) => have.trim().to_ascii_lowercase() == *w,
                _ => false,
            }
    })?;

    for (k, v) in &incoming.values {
        target.values.insert(k.clone(), v.clone());
    }
    Some(target.name.clone())
}

/// Replace the global variable layout (presentation only — never touches values).
pub fn set_layout(data: &mut ProfilesData, layout: Vec<LayoutEntry>) {
    data.layout = layout;
}

/// Replace the per-variable descriptions (empty descriptions are dropped).
pub fn set_descriptions(data: &mut ProfilesData, descriptions: BTreeMap<String, String>) {
    data.descriptions =
        descriptions.into_iter().filter(|(_, v)| !v.trim().is_empty()).collect();
}

/// Merge imported profiles by appending each with a fresh id.
pub fn merge_import(data: &mut ProfilesData, imported: ProfilesData) {
    for mut p in imported.profiles {
        p.id = gen_id();
        if p.source.trim().is_empty() {
            p.source = "import".into();
        }
        data.profiles.push(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_upsert_and_delete() {
        let mut d = ProfilesData::default();
        let mut vals = BTreeMap::new();
        vals.insert("firstName".into(), "Sam".into());
        upsert_profile(
            &mut d,
            Profile { id: String::new(), name: "Client A".into(), values: vals, source: String::new(), tone: String::new() },
        );
        assert_eq!(d.profiles.len(), 1);
        assert_eq!(d.profiles[0].source, "manual"); // defaulted
        let pid = d.profiles[0].id.clone();

        upsert_profile(
            &mut d,
            Profile {
                id: pid.clone(),
                name: "Client A+".into(),
                values: BTreeMap::new(),
                source: "manual".into(),
                tone: String::new(),
            },
        );
        assert_eq!(d.profiles.len(), 1);
        assert_eq!(d.profiles[0].name, "Client A+");

        delete_profile(&mut d, &pid);
        assert!(d.profiles.is_empty());
    }

    #[test]
    fn layout_is_independent_of_values() {
        let mut d = ProfilesData::default();
        set_layout(
            &mut d,
            vec![
                LayoutEntry { kind: "splitter".into(), label: "Contact".into(), name: String::new() },
                LayoutEntry { kind: "var".into(), label: String::new(), name: "firstName".into() },
            ],
        );
        // A webhook-style profile create/update must not disturb the layout, and the
        // layout must not disturb the values — they are fully independent.
        let mut vals = BTreeMap::new();
        vals.insert("firstName".into(), "Sam".into());
        upsert_profile(
            &mut d,
            Profile { id: String::new(), name: "Sam".into(), values: vals, source: "webhook".into(), tone: String::new() },
        );
        assert_eq!(d.layout.len(), 2);
        assert_eq!(d.layout[0].label, "Contact");
        assert_eq!(d.layout[1].name, "firstName");
        assert_eq!(d.profiles[0].values.get("firstName").unwrap(), "Sam");
    }

    fn profile(name: &str, kv: &[(&str, &str)]) -> Profile {
        let mut values = BTreeMap::new();
        for (k, v) in kv {
            values.insert((*k).into(), (*v).into());
        }
        Profile { id: gen_id(), name: name.into(), values, source: "manual".into(), tone: String::new() }
    }

    #[test]
    fn enrich_matches_by_name_case_insensitive() {
        let mut d = ProfilesData::default();
        d.profiles.push(profile("Sam Rivera", &[("company", "Acme")]));
        let incoming = profile("sam rivera", &[("phone", "+1 555 0100"), ("company", "Globex")]);
        let matched = enrich_existing(&mut d, &incoming).unwrap();
        assert_eq!(matched, "Sam Rivera");
        assert_eq!(d.profiles[0].values.get("phone").unwrap(), "+1 555 0100");
        assert_eq!(d.profiles[0].values.get("company").unwrap(), "Globex"); // overwritten
    }

    #[test]
    fn enrich_matches_by_email_when_name_differs() {
        let mut d = ProfilesData::default();
        d.profiles.push(profile("Old Name", &[("email", "sam@acme.com")]));
        let mut incoming = profile("Totally Different", &[("title", "CTO")]);
        incoming.values.insert("email".into(), "SAM@ACME.COM".into());
        let matched = enrich_existing(&mut d, &incoming).unwrap();
        assert_eq!(matched, "Old Name");
        assert_eq!(d.profiles[0].values.get("title").unwrap(), "CTO");
    }

    #[test]
    fn enrich_returns_none_without_a_match() {
        let mut d = ProfilesData::default();
        d.profiles.push(profile("Sam", &[("email", "sam@acme.com")]));
        let incoming = profile("Nobody", &[("email", "nobody@else.com")]);
        assert!(enrich_existing(&mut d, &incoming).is_none());
    }

    #[test]
    fn old_file_without_layout_still_loads() {
        // A profiles.json written before `layout` existed must deserialize cleanly.
        let json = r#"{ "profiles": [] }"#;
        let d: ProfilesData = serde_json::from_str(json).unwrap();
        assert!(d.layout.is_empty());
        assert!(d.descriptions.is_empty());
    }

    #[test]
    fn descriptions_roundtrip_and_survive_upserts() {
        let mut d = ProfilesData::default();
        let mut desc = BTreeMap::new();
        desc.insert("companyName".into(), "abbreviated lowercase company name".into());
        desc.insert("blanky".into(), "   ".into()); // dropped
        set_descriptions(&mut d, desc);
        assert_eq!(d.descriptions.len(), 1);

        upsert_profile(&mut d, profile("Sam", &[("companyName", "rocketfarm")]));
        assert_eq!(
            d.descriptions.get("companyName").unwrap(),
            "abbreviated lowercase company name"
        );

        let json = serde_json::to_string(&d).unwrap();
        let back: ProfilesData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.descriptions.len(), 1);
    }
}
