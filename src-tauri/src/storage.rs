//! Shared persistence helpers for the JSON stores (library / profiles /
//! settings / history): crash-safe writes and corrupt-file quarantine, so a
//! power loss or abort mid-write can never silently destroy user data.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Write atomically: sibling tmp file + fsync + rename. `rename` replaces the
/// destination in one step on Windows and macOS, so readers (and a crashed
/// writer) only ever see the old or the new content — never a truncated file.
pub fn write_atomic(path: &Path, content: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Append ".tmp" to the full filename (library.json -> library.json.tmp);
    // `with_extension` would eat multi-dot names.
    let mut tmp_os = path.as_os_str().to_owned();
    tmp_os.push(".tmp");
    let tmp = PathBuf::from(tmp_os);
    {
        let mut f = File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

/// Outcome of loading a JSON store from disk.
pub enum LoadedStore<T> {
    /// File existed and parsed.
    Parsed(T),
    /// File missing or unreadable — caller seeds defaults.
    Missing,
    /// File existed but didn't parse. The original was moved to `backup` so
    /// the caller's fallback-to-defaults save can never destroy it.
    Corrupt { backup: PathBuf },
}

/// Read + parse `path`, quarantining an unreadable/unparseable file instead of
/// losing it. Only a genuinely absent file reports `Missing` (the one case where
/// the caller may write seeded defaults) — every other problem is quarantined.
pub fn load_json<T: serde::de::DeserializeOwned>(path: &Path) -> LoadedStore<T> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return LoadedStore::Missing,
        // Exists but unreadable (a lock held by AV/backup, a permission error).
        // Never let the caller overwrite it with defaults — move it aside.
        Err(_) => return quarantine(path),
    };
    // `from_slice` treats invalid UTF-8 as a parse error, so a torn write that
    // splits a multi-byte character is quarantined, not silently reset.
    match serde_json::from_slice(&bytes) {
        Ok(data) => LoadedStore::Parsed(data),
        Err(_) => quarantine(path),
    }
}

/// Move a file aside to `<name>.corrupt-<ts>` so a later defaults-save can't
/// destroy it, and report where it went.
fn quarantine<T>(path: &Path) -> LoadedStore<T> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut backup_os = path.as_os_str().to_owned();
    backup_os.push(format!(".corrupt-{ts}"));
    let backup = PathBuf::from(backup_os);
    if fs::rename(path, &backup).is_err() {
        let _ = fs::copy(path, &backup);
    }
    LoadedStore::Corrupt { backup }
}

/// Human-readable problems collected while the stores loaded (before the
/// webview exists). The frontend drains these into a toast on mount.
#[derive(Default)]
pub struct StartupWarnings(pub Mutex<Vec<String>>);

/// The warning pushed when a store had to be quarantined.
pub fn corrupt_warning(store: &str, backup: &Path) -> String {
    let name = backup
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("{store}.corrupt"));
    format!("{store} couldn't be read and was reset — your original file was kept as {name}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Doc {
        a: i32,
    }

    #[test]
    fn write_atomic_replaces_and_leaves_no_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.json");
        write_atomic(&path, "{\"a\":1}").unwrap();
        write_atomic(&path, "{\"a\":2}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":2}");
        let tmp_left = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .any(|e| e.file_name().to_string_lossy().ends_with(".tmp"));
        assert!(!tmp_left, "no tmp file should remain after a write");
    }

    #[test]
    fn load_json_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            load_json::<Doc>(&dir.path().join("nope.json")),
            LoadedStore::Missing
        ));
    }

    #[test]
    fn load_json_parses_valid_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("doc.json");
        fs::write(&path, "{\"a\":7}").unwrap();
        match load_json::<Doc>(&path) {
            LoadedStore::Parsed(d) => assert_eq!(d.a, 7),
            _ => panic!("expected Parsed"),
        }
    }

    #[test]
    fn load_json_invalid_utf8_is_quarantined_not_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.json");
        // Bytes that aren't valid UTF-8 — what a torn write mid-multibyte-char
        // leaves behind. read_to_string would have mapped this to Missing.
        fs::write(&path, [0xff, 0xfe, b'{', 0x80]).unwrap();
        match load_json::<Doc>(&path) {
            LoadedStore::Corrupt { backup } => {
                assert!(!path.exists(), "original must be moved aside, not read-then-overwritten");
                assert!(backup.exists());
            }
            _ => panic!("invalid UTF-8 must quarantine, not report Missing"),
        }
    }

    #[test]
    fn load_json_corrupt_quarantines_original() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.json");
        fs::write(&path, "{ not json").unwrap();
        let backup = match load_json::<Doc>(&path) {
            LoadedStore::Corrupt { backup } => backup,
            _ => panic!("expected Corrupt"),
        };
        assert!(!path.exists(), "original should have been moved aside");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "{ not json");
        assert!(backup
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("library.json.corrupt-"));
    }
}
