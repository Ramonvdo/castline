//! Scheduled jobs — a background ticker that, on a daily/weekly/monthly
//! cadence, POSTs all profiles / one item / one folder to a connector, or
//! writes a local backup of the data files.
//!
//! Robustness rules (a machine that was off for a week must NOT blast sends
//! at login):
//!   - Missed runs are **skipped** by default: at launch, overdue schedules are
//!     re-anchored to "now" without sending (`settings::reconcile_missed`).
//!     Opting into `catch_up` fires at most ONE catch-up run.
//!   - The first tick waits ~90s after launch, then checks every 60s.
//!   - A run (success or failure) stamps `last_run`, so nothing ever fires
//!     more than once per cadence.

use std::time::Duration;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::connectors;
use crate::library::{self, LibraryState};
use crate::profiles::ProfilesState;
use crate::settings::{self, Schedule, SettingsState};

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn item_json(item: &library::LibItem) -> serde_json::Value {
    // `text` = the whole message (SOP steps stacked); `text_pages` = the same
    // separated by `---` (markdown page breaks) — automations pick either.
    let parts: Vec<&str> = if item.kind == "sop" {
        item.steps.iter().map(|s| s.text.as_str()).collect()
    } else {
        vec![item.text.as_str()]
    };
    json!({
        "name": item.name,
        "type": item.item_type,
        "kind": item.kind,
        "tags": item.tags,
        "subject": item.subject,
        "text": parts.join("\n\n"),
        "text_pages": parts.join("\n\n---\n\n"),
        "steps": item.steps.iter().map(|st| json!({ "title": st.title, "text": st.text })).collect::<Vec<_>>(),
    })
}

/// The JSON payload a webhook schedule sends.
fn build_payload(app: &AppHandle, s: &Schedule) -> Result<String, String> {
    match s.kind.as_str() {
        "item" => {
            let data = app.state::<LibraryState>().data.lock().unwrap().clone();
            let item = library::find_item(&data, &s.item_id)
                .ok_or("the scheduled item no longer exists")?;
            Ok(item_json(&item).to_string())
        }
        "folder" => {
            let data = app.state::<LibraryState>().data.lock().unwrap().clone();
            let folder = library::find_folder(&data, &s.folder_id)
                .ok_or("the scheduled folder no longer exists")?;
            Ok(json!({
                "folder": folder.name,
                "items": folder.items.iter().map(item_json).collect::<Vec<_>>(),
            })
            .to_string())
        }
        _ => {
            let data = app.state::<ProfilesState>().data.lock().unwrap().clone();
            Ok(json!({
                "profiles": data.profiles.iter().map(|p| json!({ "name": p.name, "values": p.values })).collect::<Vec<_>>(),
            })
            .to_string())
        }
    }
}

/// Write `castline-backup-YYYY-MM-DD-HHMM.json` ({ library, profiles }) to `dir`.
fn run_backup(app: &AppHandle, s: &Schedule) -> Result<String, String> {
    let dir = s.dir.trim();
    if dir.is_empty() {
        return Err("the backup schedule has no target folder".into());
    }
    let dir_path = std::path::Path::new(dir);
    std::fs::create_dir_all(dir_path).map_err(|e| format!("cannot create backup folder: {e}"))?;

    let library = app.state::<LibraryState>().data.lock().unwrap().clone();
    let profiles = app.state::<ProfilesState>().data.lock().unwrap().clone();
    let bundle = json!({ "library": library, "profiles": profiles });
    let name = format!("castline-backup-{}.json", chrono::Local::now().format("%Y-%m-%d-%H%M"));
    let path = dir_path.join(&name);
    std::fs::write(&path, serde_json::to_string_pretty(&bundle).map_err(|e| e.to_string())?)
        .map_err(|e| format!("could not write backup: {e}"))?;
    Ok(format!("Backed up library + profiles → {name}"))
}

fn stamp_last_run(app: &AppHandle, schedule_id: &str) {
    let state = app.state::<SettingsState>();
    {
        let mut cfg = state.data.lock().unwrap();
        if let Some(s) = cfg.schedules.iter_mut().find(|s| s.id == schedule_id) {
            s.last_run = now_unix();
        }
    }
    state.save();
}

/// Run one schedule now. Returns a short human description for the toast.
pub fn run_schedule(app: &AppHandle, schedule_id: &str) -> Result<String, String> {
    let schedule = {
        let snap = app.state::<SettingsState>().snapshot();
        snap.schedules
            .iter()
            .find(|s| s.id == schedule_id)
            .cloned()
            .ok_or("schedule not found")?
    };

    let result = if schedule.kind == "backup" {
        run_backup(app, &schedule)
    } else {
        let (url, cname) = {
            let snap = app.state::<SettingsState>().snapshot();
            let c = snap
                .connectors
                .iter()
                .find(|c| c.id == schedule.connector_id)
                .cloned()
                .ok_or("the schedule's connector no longer exists")?;
            if c.url.trim().is_empty() {
                return Err("the schedule's connector has no URL".into());
            }
            let name = if c.name.trim().is_empty() { c.url.clone() } else { c.name.clone() };
            (c.url, name)
        };
        let what = match schedule.kind.as_str() {
            "item" => "item",
            "folder" => "folder",
            _ => "all profiles",
        };
        let payload = build_payload(app, &schedule)?;
        let result = connectors::connector_send(&url, &payload);
        let outcome = match &result {
            Ok(r) => Ok(r.status),
            Err(e) => Err(e.clone()),
        };
        crate::log_send(app, &url, &format!("Schedule · {what}"), &payload, &outcome);
        let res = result?;
        if res.status >= 300 {
            Err(format!("connector answered {}: {}", res.status, res.body))
        } else {
            Ok(format!("Sent {what} → {cname}"))
        }
    };

    // Success or failure, the cadence restarts from this attempt — a broken
    // schedule must not retry every minute.
    stamp_last_run(app, schedule_id);
    result
}

/// Spawn the ticker: reconcile missed schedules first (skip, don't blast),
/// wait ~90s, then check every 60s for due schedules.
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        // Launch-time reconciliation: overdue + !catch_up → re-anchor silently.
        {
            let state = app.state::<SettingsState>();
            let changed = {
                let mut cfg = state.data.lock().unwrap();
                settings::reconcile_missed(&mut cfg.schedules, now_unix())
            };
            if changed {
                state.save();
            }
        }
        std::thread::sleep(Duration::from_secs(90));
        loop {
            let due: Vec<String> = {
                let snap = app.state::<SettingsState>().snapshot();
                let now = now_unix();
                snap.schedules
                    .iter()
                    .filter(|s| settings::schedule_due(s, now))
                    .map(|s| s.id.clone())
                    .collect()
            };
            for id in due {
                match run_schedule(&app, &id) {
                    Ok(msg) => {
                        let _ = app.emit("schedule-ran", msg);
                    }
                    Err(e) => {
                        let _ = app.emit("schedule-ran", format!("Scheduled job failed: {e}"));
                    }
                }
            }
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}
