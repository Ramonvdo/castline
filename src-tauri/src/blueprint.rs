//! Blueprints — a shareable JSON description of one or more templates.
//!
//! The point is sharing: you export a template (or a selection, or a whole
//! folder) to a small `.castline.json` file, hand it to someone, and they drop
//! it into their app. Same idea as a Make.com / n8n scenario blueprint.
//!
//! A blueprint carries only what is *shareable*: names, text, steps, tags. Ids,
//! use counts, pins and timestamps are personal bookkeeping and are deliberately
//! never written out — nobody needs to know how often you used a template, and a
//! foreign id would collide with the importer's library.

use serde::{Deserialize, Serialize};

use crate::library::{self, LibFolder, LibItem, SopStep};

/// Format version written into every export. Bump only on a breaking change.
pub const BLUEPRINT_VERSION: u32 = 1;

fn default_kind() -> String {
    "template".into()
}

/// One SOP step, minus its id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintStep {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub text: String,
}

/// A template stripped of everything local to one machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintItem {
    pub name: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    /// Wire key is `type` — same rename as `LibItem`, so the two stay swappable.
    #[serde(rename = "type", default)]
    pub item_type: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub steps: Vec<BlueprintStep>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Folder presentation carried by a whole-folder export, so the importer can
/// recreate it looking the same.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintFolder {
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub color: String,
}

/// The file itself.
///
/// `castline_blueprint` is required and is the ONLY reliable way to tell a
/// blueprint from any other JSON: `LibraryData::folders` is `#[serde(default)]`,
/// so a whole-library export — or even a bare `{}` — would otherwise happily
/// deserialize as an empty blueprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    #[serde(rename = "castline_blueprint")]
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<BlueprintFolder>,
    pub items: Vec<BlueprintItem>,
    #[serde(default)]
    pub exported_at: String,
    #[serde(default)]
    pub app_version: String,
    /// Informational: the `{{variables}}` these templates expect, so the file
    /// reads well on its own. Always recomputed on import — never trusted.
    #[serde(default)]
    pub variables: Vec<String>,
}

/// Build a blueprint from live library items, dropping every personal field.
pub fn from_items(items: &[LibItem], folder: Option<&LibFolder>, app_version: &str) -> Blueprint {
    let items: Vec<BlueprintItem> = items
        .iter()
        .map(|i| BlueprintItem {
            name: i.name.clone(),
            kind: i.kind.clone(),
            item_type: i.item_type.clone(),
            text: i.text.clone(),
            subject: i.subject.clone(),
            steps: i
                .steps
                .iter()
                .map(|s| BlueprintStep { title: s.title.clone(), text: s.text.clone() })
                .collect(),
            tags: i.tags.clone(),
        })
        .collect();
    Blueprint {
        version: BLUEPRINT_VERSION,
        folder: folder.map(|f| BlueprintFolder {
            name: f.name.clone(),
            icon: f.icon.clone(),
            color: f.color.clone(),
        }),
        variables: item_variables(&items),
        items,
        exported_at: library::now_iso(),
        app_version: app_version.to_string(),
    }
}

/// Parse + validate, with errors written for the person who dropped the file in.
pub fn parse(text: &str) -> Result<Blueprint, String> {
    // Probe the discriminator first so "this is the wrong kind of file" reads
    // like that, instead of a serde field-missing error.
    let probe: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| format!("That file isn't valid JSON: {e}"))?;
    if probe.get("castline_blueprint").is_none() {
        return Err("This file isn't a Castline blueprint.".into());
    }

    let bp: Blueprint =
        serde_json::from_str(text).map_err(|e| format!("Not a valid Castline blueprint: {e}"))?;
    if bp.version > BLUEPRINT_VERSION {
        return Err("This blueprint was made with a newer version of Castline.".into());
    }
    if bp.items.is_empty() {
        return Err("This blueprint has no templates in it.".into());
    }
    Ok(bp)
}

/// Convert to library items ready for `upsert_item`.
///
/// Every id is left blank on purpose: `library::normalize_item` then stamps a
/// fresh id, fresh step ids and timestamps, so an imported template can never
/// collide with — or silently overwrite — something already in the library.
pub fn to_lib_items(bp: &Blueprint) -> Vec<LibItem> {
    bp.items
        .iter()
        .map(|b| LibItem {
            id: String::new(),
            name: b.name.clone(),
            kind: if b.kind == "sop" { "sop".into() } else { "template".into() },
            item_type: b.item_type.clone(),
            text: b.text.clone(),
            subject: b.subject.clone(),
            steps: b
                .steps
                .iter()
                .map(|s| SopStep {
                    id: String::new(),
                    title: s.title.clone(),
                    text: s.text.clone(),
                })
                .collect(),
            tags: b.tags.clone(),
            favorite: false,
            uses: 0,
            created_at: String::new(),
            updated_at: String::new(),
        })
        .collect()
}

/// The `{{variables}}` these items expect, first-seen order, auto tokens
/// (`{{today}}`, `{{now:HH:mm}}`) excluded — the blueprint twin of
/// `library::all_vars`.
pub fn item_variables(items: &[BlueprintItem]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut scan = |text: &str| {
        let mut rest = text;
        while let Some(start) = rest.find("{{") {
            let after = &rest[start + 2..];
            let Some(end) = after.find("}}") else { break };
            let name = after[..end].trim();
            if !name.is_empty()
                && !name.contains('{')
                && !name.contains('}')
                && !library::is_auto_var(name)
                && !out.iter().any(|v| v == name)
            {
                out.push(name.to_string());
            }
            rest = &after[end + 2..];
        }
    };
    for item in items {
        scan(&item.subject);
        scan(&item.text);
        for step in &item.steps {
            scan(&step.text);
        }
    }
    out
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn lib_item(name: &str) -> LibItem {
        LibItem {
            id: "local-id".into(),
            name: name.into(),
            kind: "template".into(),
            item_type: "email".into(),
            subject: "Quick idea for {{companyName}}".into(),
            text: "Hey {{firstName}}, about {{companyName}} — sent {{today}}".into(),
            steps: vec![],
            tags: vec!["sales".into()],
            favorite: true,
            uses: 42,
            created_at: "2026-01-01T00:00:00".into(),
            updated_at: "2026-01-02T00:00:00".into(),
        }
    }

    #[test]
    fn roundtrip_preserves_shareable_fields() {
        let bp = from_items(&[lib_item("Cold outreach")], None, "1.1.3");
        let text = serde_json::to_string_pretty(&bp).unwrap();
        let back = parse(&text).unwrap();

        assert_eq!(back.version, BLUEPRINT_VERSION);
        assert_eq!(back.items.len(), 1);
        let it = &back.items[0];
        assert_eq!(it.name, "Cold outreach");
        assert_eq!(it.kind, "template");
        // The serde rename survives: `item_type` <-> the "type" wire key.
        assert_eq!(it.item_type, "email");
        assert!(text.contains("\"type\": \"email\""));
        assert_eq!(it.subject, "Quick idea for {{companyName}}");
        assert_eq!(it.tags, vec!["sales".to_string()]);
    }

    #[test]
    fn export_strips_personal_fields() {
        let bp = from_items(&[lib_item("A")], None, "1.1.3");
        let text = serde_json::to_string(&bp).unwrap();
        for leaked in ["local-id", "uses", "favorite", "created_at", "updated_at"] {
            assert!(!text.contains(leaked), "blueprint leaked `{leaked}`: {text}");
        }
    }

    #[test]
    fn imported_items_are_blanked_for_renormalisation() {
        let bp = from_items(&[lib_item("A")], None, "1.1.3");
        let items = to_lib_items(&bp);
        assert_eq!(items.len(), 1);
        let it = &items[0];
        assert!(it.id.is_empty());
        assert_eq!(it.uses, 0);
        assert!(!it.favorite);
        assert!(it.created_at.is_empty());
        assert!(it.updated_at.is_empty());
    }

    #[test]
    fn sop_steps_survive_with_blank_ids() {
        let mut src = lib_item("Blog SOP");
        src.kind = "sop".into();
        src.steps = vec![
            SopStep { id: "s1".into(), title: "Outline".into(), text: "Outline {{topic}}".into() },
            SopStep { id: "s2".into(), title: "Draft".into(), text: "Write {{topic}}".into() },
        ];
        let bp = from_items(&[src], None, "1.1.3");
        assert!(!serde_json::to_string(&bp).unwrap().contains("\"s1\""));

        let items = to_lib_items(&bp);
        assert_eq!(items[0].kind, "sop");
        assert_eq!(items[0].steps.len(), 2);
        assert_eq!(items[0].steps[0].title, "Outline");
        assert!(items[0].steps.iter().all(|s| s.id.is_empty()));
    }

    #[test]
    fn folder_metadata_rides_along() {
        let folder = LibFolder {
            id: "f1".into(),
            name: "Sales".into(),
            color: "#6fa8c9".into(),
            icon: "mail".into(),
            items: vec![],
        };
        let bp = from_items(&[lib_item("A")], Some(&folder), "1.1.3");
        let back = parse(&serde_json::to_string(&bp).unwrap()).unwrap();
        let f = back.folder.expect("folder metadata missing");
        assert_eq!(f.name, "Sales");
        assert_eq!(f.icon, "mail");
        assert_eq!(f.color, "#6fa8c9");
    }

    #[test]
    fn a_library_file_is_not_a_blueprint() {
        // The trap: LibraryData::folders is #[serde(default)], so both of these
        // would deserialize as an empty Blueprint without the discriminator.
        assert!(parse(r#"{"folders":[]}"#).is_err());
        assert!(parse("{}").is_err());
        let err = parse(r#"{"folders":[]}"#).unwrap_err();
        assert!(err.contains("isn't a Castline blueprint"), "got: {err}");
    }

    #[test]
    fn newer_version_is_rejected() {
        let err = parse(r#"{"castline_blueprint":2,"items":[{"name":"x"}]}"#).unwrap_err();
        assert!(err.contains("newer version"), "got: {err}");
    }

    #[test]
    fn empty_items_rejected() {
        let err = parse(r#"{"castline_blueprint":1,"items":[]}"#).unwrap_err();
        assert!(err.contains("no templates"), "got: {err}");
    }

    #[test]
    fn unknown_fields_are_tolerated() {
        // Forward compat: a file from a future minor release must still import.
        let bp = parse(
            r#"{"castline_blueprint":1,"items":[{"name":"x","surprise":true}],"whats_this":42}"#,
        )
        .unwrap();
        assert_eq!(bp.items[0].name, "x");
        // Omitted fields fall back to their defaults.
        assert_eq!(bp.items[0].kind, "template");
        assert!(bp.items[0].text.is_empty());
    }

    #[test]
    fn variables_are_ordered_deduped_and_skip_auto_tokens() {
        let items = vec![BlueprintItem {
            name: "A".into(),
            kind: "template".into(),
            item_type: String::new(),
            subject: "Hi {{firstName}}".into(),
            text: "{{firstName}} at {{company}} on {{today}} at {{now:HH:mm}}".into(),
            steps: vec![BlueprintStep { title: "s".into(), text: "{{goal}}".into() }],
            tags: vec![],
        }];
        assert_eq!(
            item_variables(&items),
            vec!["firstName".to_string(), "company".to_string(), "goal".to_string()]
        );
    }
}
