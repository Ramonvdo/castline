//! "Start with Windows" — which works in two completely different ways depending
//! on how Castline was installed.
//!
//! A plain `.exe` install registers a `Run` key (tauri-plugin-autostart). An MSIX
//! install *cannot*: Windows virtualizes that registry write into the package's
//! private hive, so the write appears to succeed and nothing ever launches. A
//! packaged build has to drive the `StartupTask` declared in
//! `Package.appxmanifest` instead.
//!
//! The user can also veto a StartupTask from Windows Settings > Apps > Startup,
//! and once they do, the app is not allowed to switch it back on. That's a
//! deliberate Windows rule, so it's surfaced (`locked`) rather than hidden —
//! a toggle that silently does nothing is worse than no toggle.

use serde::Serialize;

/// What the UI needs to render an honest control.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Status {
    /// Windows owns this setting (packaged/MSIX build).
    pub managed: bool,
    pub enabled: bool,
    /// The user or policy decided; the app can't override it.
    pub locked: bool,
}

#[cfg(windows)]
mod imp {
    use super::Status;
    use windows::core::HSTRING;
    use windows::ApplicationModel::{StartupTask, StartupTaskState};

    /// Must match the TaskId in Package.appxmanifest.
    const TASK_ID: &str = "CastlineStartup";

    /// `None` for an unpackaged build — asking for a StartupTask without package
    /// identity fails, which doubles as the packaged/unpackaged check.
    fn task() -> Option<StartupTask> {
        StartupTask::GetAsync(&HSTRING::from(TASK_ID)).ok()?.get().ok()
    }

    fn read(t: &StartupTask) -> Status {
        let s = t.State().unwrap_or(StartupTaskState::Disabled);
        let enabled = s == StartupTaskState::Enabled || s == StartupTaskState::EnabledByPolicy;
        let locked = s == StartupTaskState::DisabledByUser
            || s == StartupTaskState::DisabledByPolicy
            || s == StartupTaskState::EnabledByPolicy;
        Status { managed: true, enabled, locked }
    }

    pub fn status() -> Option<Status> {
        task().as_ref().map(read)
    }

    pub fn set(enabled: bool) -> Option<Status> {
        let t = task()?;
        if enabled {
            // Returns the resulting state; a user-disabled task stays disabled.
            let _ = t.RequestEnableAsync().ok()?.get();
        } else {
            let _ = t.Disable();
        }
        Some(read(&t))
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Status;
    pub fn status() -> Option<Status> {
        None
    }
    pub fn set(_enabled: bool) -> Option<Status> {
        None
    }
}

/// Current state. `managed: false` means this is a plain install and the caller
/// should fall back to the autostart plugin.
pub fn status(plugin_enabled: bool) -> Status {
    imp::status().unwrap_or(Status { managed: false, enabled: plugin_enabled, locked: false })
}

/// Apply, when Windows manages startup. `None` means it doesn't — use the plugin.
pub fn set(enabled: bool) -> Option<Status> {
    imp::set(enabled)
}

/// Is this a packaged (MSIX / Microsoft Store) build? Package identity is what
/// makes the StartupTask lookup succeed at all, so the same probe answers both
/// questions. Used to label Agent-tab diagnostics, where Store installs behave
/// differently from a plain `.exe`.
pub fn is_packaged() -> bool {
    imp::status().is_some()
}
