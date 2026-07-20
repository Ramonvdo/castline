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
    /// LEGACY (pre-global locks): per-profile locked variables. Migrated into
    /// `ProfilesData.locked` on load; kept only so old files deserialize.
    #[serde(default)]
    pub locked: Vec<String>,
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
    /// GLOBAL locked-empty variables: locked once, locked for every profile.
    /// They must be filled on the spot and are never written by any enrich
    /// path (AI, webhook, inbound API) or profile save.
    #[serde(default)]
    pub locked: Vec<String>,
}

/// Tauri-managed live profiles + the JSON path it persists to.
pub struct ProfilesState {
    pub data: Mutex<ProfilesData>,
    pub path: PathBuf,
}

impl ProfilesState {
    pub fn load(path: PathBuf, warnings: &mut Vec<String>) -> Self {
        let mut data = match crate::storage::load_json::<ProfilesData>(&path) {
            crate::storage::LoadedStore::Parsed(d) => d,
            crate::storage::LoadedStore::Corrupt { backup } => {
                warnings.push(crate::storage::corrupt_warning("profiles.json", &backup));
                ProfilesData::default()
            }
            crate::storage::LoadedStore::Missing => {
                let d = ProfilesData::default();
                if let Ok(json) = serde_json::to_string_pretty(&d) {
                    let _ = crate::storage::write_atomic(&path, &json);
                }
                d
            }
        };
        // Pre-global-locks files carry per-profile lock lists — fold them into
        // the global list once and persist immediately.
        if migrate_locked(&mut data) {
            if let Ok(json) = serde_json::to_string_pretty(&data) {
                let _ = crate::storage::write_atomic(&path, &json);
            }
        }
        ProfilesState { data: Mutex::new(data), path }
    }

    pub fn save(&self) {
        if let Ok(data) = self.data.lock() {
            if let Ok(json) = serde_json::to_string_pretty(&*data) {
                let _ = crate::storage::write_atomic(&self.path, &json);
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

/// One-time migration: union per-profile `locked` lists (pre-global-locks
/// files) into the global `data.locked` and clear the legacy fields.
/// Returns true when anything moved.
pub fn migrate_locked(data: &mut ProfilesData) -> bool {
    let mut changed = false;
    let mut collected: Vec<String> = Vec::new();
    for p in &mut data.profiles {
        collected.append(&mut p.locked);
        if !collected.is_empty() {
            changed = true;
        }
    }
    for l in collected {
        if !data.locked.contains(&l) {
            data.locked.push(l);
        }
    }
    changed
}

/// Insert (blank id) or replace (matching id) a profile. Globally-locked
/// variables are stripped from the values on EVERY write path (frontend
/// saves, inbound create/update, connector imports) — locked means empty
/// in every profile, enforced here so no caller can forget.
pub fn upsert_profile(data: &mut ProfilesData, mut profile: Profile) {
    if profile.id.trim().is_empty() {
        profile.id = gen_id();
    }
    if profile.source.trim().is_empty() {
        profile.source = "manual".into();
    }
    profile.values.retain(|k, _| !data.locked.iter().any(|l| l == k));
    profile.locked.clear(); // legacy per-profile field, superseded by data.locked
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

    let locked = data.locked.clone();
    let target = data.profiles.iter_mut().find(|p| {
        (!want_name.is_empty() && p.name.trim().to_ascii_lowercase() == want_name)
            || match (&want_email, p.values.get("email")) {
                (Some(w), Some(have)) => have.trim().to_ascii_lowercase() == *w,
                _ => false,
            }
    })?;

    for (k, v) in &incoming.values {
        // Globally-locked variables must always be filled on the spot — no
        // enrich path may write them, in any profile.
        if locked.iter().any(|l| l == k) {
            continue;
        }
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

/// Replace the GLOBAL locked-variable list. Locking a variable clears its
/// current value in every profile (locked = empty everywhere, filled on the
/// spot); unlocking simply allows values again.
pub fn set_locked(data: &mut ProfilesData, locked: Vec<String>) {
    let mut clean: Vec<String> = Vec::new();
    for l in locked {
        let t = l.trim().to_string();
        if !t.is_empty() && !clean.contains(&t) {
            clean.push(t);
        }
    }
    for p in &mut data.profiles {
        p.values.retain(|k, _| !clean.iter().any(|l| l == k));
    }
    data.locked = clean;
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
            Profile { id: String::new(), name: "Client A".into(), values: vals, source: String::new(), tone: String::new(), locked: Vec::new() },
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
                locked: Vec::new(),
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
            Profile { id: String::new(), name: "Sam".into(), values: vals, source: "webhook".into(), tone: String::new(), locked: Vec::new() },
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
        Profile { id: gen_id(), name: name.into(), values, source: "manual".into(), tone: String::new(), locked: Vec::new() }
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
    fn enrich_never_writes_globally_locked_variables() {
        let mut d = ProfilesData::default();
        d.locked = vec!["icebreaker".into(), "company".into()];
        d.profiles.push(profile("Sam", &[("company", "Acme")]));

        let incoming =
            profile("Sam", &[("icebreaker", "generated text"), ("company", "Globex"), ("title", "CTO")]);
        enrich_existing(&mut d, &incoming).unwrap();
        // Locked keys untouched (icebreaker stays absent, company keeps its value)…
        assert!(d.profiles[0].values.get("icebreaker").is_none());
        assert_eq!(d.profiles[0].values.get("company").unwrap(), "Acme");
        // …while unlocked keys merge normally.
        assert_eq!(d.profiles[0].values.get("title").unwrap(), "CTO");

        // Round-trips through JSON.
        let json = serde_json::to_string(&d).unwrap();
        let back: ProfilesData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.locked.len(), 2);
    }

    #[test]
    fn locking_is_global_clears_values_and_blocks_saves() {
        let mut d = ProfilesData::default();
        d.profiles.push(profile("Sam", &[("icebreaker", "hey"), ("company", "Acme")]));
        d.profiles.push(profile("Kim", &[("icebreaker", "hi there")]));

        // Locking clears the variable in EVERY profile, not just one.
        set_locked(&mut d, vec!["icebreaker".into(), " icebreaker ".into(), "".into()]);
        assert_eq!(d.locked, vec!["icebreaker".to_string()]); // trimmed + deduped
        assert!(d.profiles[0].values.get("icebreaker").is_none());
        assert!(d.profiles[1].values.get("icebreaker").is_none());
        assert_eq!(d.profiles[0].values.get("company").unwrap(), "Acme");

        // Any later save (frontend, inbound create, connector import) is
        // stripped of locked values too.
        upsert_profile(&mut d, profile("New Guy", &[("icebreaker", "smuggled"), ("title", "CTO")]));
        let np = d.profiles.iter().find(|p| p.name == "New Guy").unwrap();
        assert!(np.values.get("icebreaker").is_none());
        assert_eq!(np.values.get("title").unwrap(), "CTO");

        // Unlocking allows values again.
        set_locked(&mut d, Vec::new());
        upsert_profile(&mut d, profile("Later", &[("icebreaker", "fine now")]));
        let lp = d.profiles.iter().find(|p| p.name == "Later").unwrap();
        assert_eq!(lp.values.get("icebreaker").unwrap(), "fine now");
    }

    #[test]
    fn per_profile_locks_migrate_into_the_global_list() {
        // A profiles.json from the per-profile-locks era.
        let json = r#"{ "profiles": [
            { "id": "a", "name": "Sam", "values": {}, "locked": ["icebreaker"] },
            { "id": "b", "name": "Kim", "values": {}, "locked": ["icebreaker", "phone"] }
        ] }"#;
        let mut d: ProfilesData = serde_json::from_str(json).unwrap();
        assert!(migrate_locked(&mut d));
        assert_eq!(d.locked, vec!["icebreaker".to_string(), "phone".to_string()]);
        assert!(d.profiles.iter().all(|p| p.locked.is_empty()));
        // Idempotent: a second pass changes nothing.
        assert!(!migrate_locked(&mut d));
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
