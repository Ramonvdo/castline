//! Scheduled outbound webhooks — a background ticker that POSTs all profiles
//! (or one library item) to a connector every day/week/month. Runs while the
//! app is open; anything overdue fires on the first tick after launch.

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

/// The JSON payload a schedule sends.
fn build_payload(app: &AppHandle, s: &Schedule) -> Result<String, String> {
    if s.kind == "item" {
        let data = app.state::<LibraryState>().data.lock().unwrap().clone();
        let item = library::find_item(&data, &s.item_id)
            .ok_or("the scheduled item no longer exists")?;
        Ok(json!({
            "name": item.name,
            "type": item.item_type,
            "kind": item.kind,
            "tags": item.tags,
            "text": item.text,
            "steps": item.steps.iter().map(|st| json!({ "title": st.title, "text": st.text })).collect::<Vec<_>>(),
        })
        .to_string())
    } else {
        let data = app.state::<ProfilesState>().data.lock().unwrap().clone();
        Ok(json!({
            "profiles": data.profiles.iter().map(|p| json!({ "name": p.name, "values": p.values })).collect::<Vec<_>>(),
        })
        .to_string())
    }
}

/// Run one schedule now: build the payload, POST it to the connector, stamp
/// `last_run`. Returns a short human description for the toast.
pub fn run_schedule(app: &AppHandle, schedule_id: &str) -> Result<String, String> {
    let (schedule, url, cname) = {
        let snap = app.state::<SettingsState>().snapshot();
        let s = snap
            .schedules
            .iter()
            .find(|s| s.id == schedule_id)
            .cloned()
            .ok_or("schedule not found")?;
        let c = snap
            .connectors
            .iter()
            .find(|c| c.id == s.connector_id)
            .cloned()
            .ok_or("the schedule's connector no longer exists")?;
        if c.url.trim().is_empty() {
            return Err("the schedule's connector has no URL".into());
        }
        let name = if c.name.trim().is_empty() { c.url.clone() } else { c.name.clone() };
        (s, c.url, name)
    };

    let payload = build_payload(app, &schedule)?;
    let res = connectors::connector_send(&url, &payload)?;
    if res.status >= 300 {
        return Err(format!("connector answered {}: {}", res.status, res.body));
    }

    // Stamp last_run so the cadence restarts from this send.
    let state = app.state::<SettingsState>();
    {
        let mut cfg = state.data.lock().unwrap();
        if let Some(s) = cfg.schedules.iter_mut().find(|s| s.id == schedule_id) {
            s.last_run = now_unix();
        }
    }
    state.save();

    let what = if schedule.kind == "item" { "item" } else { "all profiles" };
    Ok(format!("Sent {what} → {cname}"))
}

/// Spawn the ticker: check every 60s for due schedules and run them.
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || loop {
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
                    // Stamp failures too so a broken schedule doesn't retry
                    // every minute — it'll try again next cadence.
                    let state = app.state::<SettingsState>();
                    {
                        let mut cfg = state.data.lock().unwrap();
                        if let Some(s) = cfg.schedules.iter_mut().find(|s| s.id == id) {
                            s.last_run = now_unix();
                        }
                    }
                    state.save();
                    let _ = app.emit("schedule-ran", format!("Scheduled send failed: {e}"));
                }
            }
        }
        std::thread::sleep(Duration::from_secs(60));
    });
}
