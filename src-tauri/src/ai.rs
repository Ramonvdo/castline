//! Embedded AI terminal: spawns the user's own `claude` CLI inside a real PTY
//! (ConPTY on Windows, openpty elsewhere) with cwd = the Castline data dir, so
//! Claude Code launches with the generated CLAUDE.md in scope.
//!
//! Data flow: xterm.js (frontend) → `ai_input` command → PTY writer;
//! PTY reader thread → coalescing emitter thread → "ai-output" events → xterm.
//! Claude's TUI redraws are bursty, so raw chunks are batched (~16 ms / 32 KiB)
//! before crossing the Tauri IPC boundary, and only valid-UTF-8 prefixes are
//! emitted (a multibyte sequence split across reads carries over to the next
//! batch instead of becoming U+FFFD).

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// One live terminal session. Dropping it (after `kill`) closes the PTY.
pub struct AiSession {
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn MasterPty + Send>,
    pub killer: Box<dyn ChildKiller + Send + Sync>,
    /// Which `start` call owns this session — stale reader/waiter threads from a
    /// restarted session compare against this before emitting or clearing.
    pub generation: u64,
}

/// Tauri-managed state for the (single) AI terminal session.
pub struct AiState {
    pub session: Mutex<Option<AiSession>>,
    pub generation: AtomicU64,
}

impl Default for AiState {
    fn default() -> Self {
        Self { session: Mutex::new(None), generation: AtomicU64::new(0) }
    }
}

/// How `claude` was located. Surfaced in `ai_status` so a user whose Agent tab
/// says "not found" can tell a missing install from a PATH problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// An explicit path set in Settings.
    Configured,
    /// Found on the PATH this process inherited.
    Path,
    /// Found by checking known install locations directly.
    Probe,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Configured => "configured",
            Source::Path => "path",
            Source::Probe => "probe",
        }
    }
}

/// A launchable claude: what to exec, its leading args, the file actually found
/// (before any shim wrapping), and how it was found.
pub struct Resolved {
    pub program: String,
    pub args: Vec<String>,
    pub found: String,
    pub source: Source,
}

/// Resolve how to launch claude. An explicit configured path wins, then the
/// inherited PATH, then a direct sweep of the locations the common installers
/// use.
///
/// That last fallback is not belt-and-braces: a packaged (MSIX/Store) build is
/// activated by the shell broker and can inherit a PATH without the user's own
/// additions, so `where.exe` finds nothing on a machine where claude is plainly
/// installed — the app then reports "not found" and looks broken.
pub fn resolve_claude(cfg: &crate::settings::AiConfig) -> Option<Resolved> {
    let explicit = cfg.claude_path.trim();
    if !explicit.is_empty() {
        let (program, args) = wrap_shim(explicit);
        return Some(Resolved {
            program,
            args,
            found: explicit.to_string(),
            source: Source::Configured,
        });
    }
    let (found, source) = match which_claude() {
        Some(p) => (p, Source::Path),
        None => (probe_claude()?, Source::Probe),
    };
    let (program, args) = wrap_shim(&found);
    Some(Resolved { program, args, found, source })
}

/// The locations the common installers drop `claude` into, in preference order.
/// Public so `ai_status` can report each one's hit/miss as diagnostics.
pub fn probe_candidates() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let home = dirs::home_dir();

    #[cfg(windows)]
    {
        // Native installer first — a real .exe beats a shim.
        if let Some(h) = &home {
            out.push(h.join(".local").join("bin").join("claude.exe"));
            out.push(h.join(".bun").join("bin").join("claude.exe"));
        }
        if let Some(local) = dirs::data_local_dir() {
            out.push(local.join("Programs").join("claude").join("claude.exe"));
            out.push(local.join("pnpm").join("claude.cmd"));
            out.push(local.join("Yarn").join("bin").join("claude.cmd"));
        }
        // npm global shim.
        if let Some(roaming) = dirs::data_dir() {
            out.push(roaming.join("npm").join("claude.cmd"));
        }
    }

    #[cfg(not(windows))]
    {
        if let Some(h) = &home {
            out.push(h.join(".local").join("bin").join("claude"));
            out.push(h.join(".bun").join("bin").join("claude"));
            out.push(h.join(".npm-global").join("bin").join("claude"));
        }
        out.push(std::path::PathBuf::from("/usr/local/bin/claude"));
        out.push(std::path::PathBuf::from("/opt/homebrew/bin/claude"));
    }

    let _ = &home;
    out
}

/// First candidate that actually exists on disk.
fn probe_claude() -> Option<String> {
    probe_candidates()
        .into_iter()
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
}

#[cfg(windows)]
fn which_claude() -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("where.exe")
        .arg("claude")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    pick_windows_match(&text)
}

/// Choose a launchable binary among `where claude` matches. npm installs ship
/// both an extensionless bash script (often listed first, but CreateProcess
/// can't run it) and a `claude.cmd` shim — prefer a real .exe, then a shim,
/// and only then fall back to the first line.
#[cfg(any(windows, test))]
fn pick_windows_match(where_output: &str) -> Option<String> {
    let lines: Vec<&str> =
        where_output.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if let Some(exe) = lines.iter().find(|l| l.to_ascii_lowercase().ends_with(".exe")) {
        return Some(exe.to_string());
    }
    if let Some(shim) = lines.iter().find(|l| {
        let l = l.to_ascii_lowercase();
        l.ends_with(".cmd") || l.ends_with(".bat")
    }) {
        return Some(shim.to_string());
    }
    lines.first().map(|p| p.to_string())
}

#[cfg(not(windows))]
fn which_claude() -> Option<String> {
    let out = std::process::Command::new("sh").args(["-lc", "command -v claude"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

/// `.cmd`/`.bat` shims must run through `cmd.exe /c`; everything else launches
/// directly.
fn wrap_shim(path: &str) -> (String, Vec<String>) {
    let lower = path.to_ascii_lowercase();
    if cfg!(windows) && (lower.ends_with(".cmd") || lower.ends_with(".bat")) {
        ("cmd.exe".into(), vec!["/c".into(), path.into()])
    } else {
        (path.into(), Vec::new())
    }
}

/// Spawn `program args` in a fresh PTY at `cwd` and wire up the reader/emitter/
/// waiter threads. Any previous session is killed first.
pub fn start(
    app: &AppHandle,
    program: &str,
    args: &[String],
    cwd: &Path,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    stop(app);

    let pty = native_pty_system();
    let size = PtySize { rows: rows.max(2), cols: cols.max(2), pixel_width: 0, pixel_height: 0 };
    let pair = pty.openpty(size).map_err(|e| e.to_string())?;

    let mut cmd = CommandBuilder::new(program);
    cmd.args(args);
    cmd.cwd(cwd);
    cmd.env("LANG", "en_US.UTF-8");

    let mut child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    // Only master + child stay alive; the slave handle must drop after spawn or
    // the reader never sees EOF when the child exits.
    drop(pair.slave);

    let killer = child.clone_killer();
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    let state = app.state::<AiState>();
    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;

    // Reader thread: blocking PTY reads → channel (closes on EOF).
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Emitter thread: coalesce bursts, emit UTF-8-safe batches.
    let emit_app = app.clone();
    std::thread::spawn(move || {
        let mut carry: Vec<u8> = Vec::new();
        loop {
            match rx.recv() {
                Ok(chunk) => carry.extend_from_slice(&chunk),
                Err(_) => break, // reader gone (EOF) — flush below then exit
            }
            let deadline = Instant::now() + Duration::from_millis(16);
            while carry.len() < 32 * 1024 {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    break;
                }
                match rx.recv_timeout(left) {
                    Ok(chunk) => carry.extend_from_slice(&chunk),
                    Err(_) => break,
                }
            }
            let valid = match std::str::from_utf8(&carry) {
                Ok(_) => carry.len(),
                Err(e) => e.valid_up_to(),
            };
            if valid == 0 {
                continue;
            }
            if emit_app.state::<AiState>().generation.load(Ordering::SeqCst) != generation {
                break; // a newer session took over
            }
            let text = String::from_utf8_lossy(&carry[..valid]).into_owned();
            let _ = emit_app.emit("ai-output", text);
            carry.drain(..valid);
        }
    });

    // Waiter thread: reap the child, then clear the session + notify the UI
    // (only if a restart hasn't already replaced this generation).
    let wait_app = app.clone();
    std::thread::spawn(move || {
        let code = child.wait().map(|s| s.exit_code()).unwrap_or(1);
        let st = wait_app.state::<AiState>();
        let mut session = st.session.lock().unwrap();
        if session.as_ref().map(|s| s.generation) == Some(generation) {
            *session = None;
            drop(session);
            let _ = wait_app.emit("ai-exit", code);
        }
    });

    let mut session = state.session.lock().map_err(|_| "ai lock poisoned".to_string())?;
    *session = Some(AiSession { writer, master: pair.master, killer, generation });
    Ok(())
}

/// Kill the current session, if any. Dropping the taken session closes the
/// master side, which tears down the ConPTY/openpty pair.
pub fn stop(app: &AppHandle) {
    let st = app.state::<AiState>();
    let taken = st.session.lock().ok().and_then(|mut s| s.take());
    if let Some(mut s) = taken {
        let _ = s.killer.kill();
    }
}

/// Forward keyboard input from xterm.js to the PTY.
pub fn input(app: &AppHandle, data: &str) -> Result<(), String> {
    let st = app.state::<AiState>();
    let mut session = st.session.lock().map_err(|_| "ai lock poisoned".to_string())?;
    let s = session.as_mut().ok_or("no AI session running")?;
    s.writer.write_all(data.as_bytes()).map_err(|e| e.to_string())
}

/// Propagate an xterm.js resize to the PTY so the TUI reflows.
pub fn resize(app: &AppHandle, rows: u16, cols: u16) -> Result<(), String> {
    let st = app.state::<AiState>();
    let session = st.session.lock().map_err(|_| "ai lock poisoned".to_string())?;
    if let Some(s) = session.as_ref() {
        s.master
            .resize(PtySize {
                rows: rows.max(2),
                cols: cols.max(2),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn is_running(app: &AppHandle) -> bool {
    let st = app.state::<AiState>();
    st.session.lock().map(|s| s.is_some()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_shim_routes_cmd_through_cmd_exe() {
        let (prog, args) = wrap_shim("C:/npm/claude.CMD");
        if cfg!(windows) {
            assert_eq!(prog, "cmd.exe");
            assert_eq!(args, vec!["/c".to_string(), "C:/npm/claude.CMD".to_string()]);
        } else {
            assert_eq!(prog, "C:/npm/claude.CMD");
            assert!(args.is_empty());
        }

        let (prog, args) = wrap_shim("C:/bin/claude.exe");
        assert_eq!(prog, "C:/bin/claude.exe");
        assert!(args.is_empty());
    }

    #[test]
    fn pick_windows_match_skips_extensionless_bash_script() {
        // npm layout: the unrunnable bash script lists before the .cmd shim.
        let out = "C:\\Users\\u\\AppData\\Roaming\\npm\\claude\r\n\
                   C:\\Users\\u\\AppData\\Roaming\\npm\\claude.cmd\r\n";
        assert!(pick_windows_match(out).unwrap().ends_with("claude.cmd"));

        // A native-install .exe outranks everything.
        let out = "C:\\npm\\claude.cmd\r\nC:\\Users\\u\\.local\\bin\\claude.exe\r\n";
        assert_eq!(pick_windows_match(out).unwrap(), "C:\\Users\\u\\.local\\bin\\claude.exe");

        assert!(pick_windows_match("").is_none());
    }

    #[test]
    fn configured_path_wins_and_reports_its_source() {
        let cfg = crate::settings::AiConfig {
            claude_path: "C:/tools/claude.exe".into(),
            extra_args: vec![],
        };
        let r = resolve_claude(&cfg).unwrap();
        assert_eq!(r.source, Source::Configured);
        assert_eq!(r.found, "C:/tools/claude.exe");
        assert_eq!(r.program, "C:/tools/claude.exe");

        // Whitespace-only is treated as unset, not as a path.
        let cfg = crate::settings::AiConfig { claude_path: "   ".into(), extra_args: vec![] };
        let resolved = resolve_claude(&cfg);
        assert!(resolved.is_none_or(|r| r.source != Source::Configured));
    }

    #[test]
    fn probe_prefers_native_install_over_npm_shim() {
        let c = probe_candidates();
        assert!(!c.is_empty(), "there should always be candidates to probe");

        let pos = |needle: &str| c.iter().position(|p| p.to_string_lossy().contains(needle));
        if let (Some(native), Some(npm)) = (pos(".local"), pos("npm")) {
            assert!(native < npm, "the native install must be probed before the npm shim");
        }
        // Every candidate is absolute — a relative probe would resolve against
        // the process cwd, which is not where anyone installs a CLI.
        assert!(c.iter().all(|p| p.is_absolute()));
    }
}
