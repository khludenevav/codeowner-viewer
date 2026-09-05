//! Shared helpers for atomic file writes + timestamped backups.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use chrono::Local;

/// Create a `<file>.bak-<timestamp>` next to `file` before mutation.
/// Returns the backup path.
pub fn backup(file: &Path) -> anyhow::Result<PathBuf> {
    if !file.exists() {
        return Ok(PathBuf::new());
    }
    let ts = Local::now().format("%Y%m%d-%H%M%S");
    let mut name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".into());
    name.push_str(&format!(".bak-{ts}"));
    let backup_path = file
        .parent()
        .map(|p| p.join(&name))
        .unwrap_or_else(|| PathBuf::from(&name));
    fs::copy(file, &backup_path)?;
    Ok(backup_path)
}

/// Write `contents` to `file` via a temp-file rename so partial writes
/// can't corrupt the target.
pub fn atomic_write(file: &Path, contents: &str) -> anyhow::Result<()> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut tmp = file.to_path_buf();
    let file_name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".into());
    tmp.set_file_name(format!(".{file_name}.tmp"));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, file)?;
    Ok(())
}

/// Format a `Value` back into pretty JSON, appending a trailing newline
/// (matching common editor conventions).
pub fn pretty_json(value: &serde_json::Value) -> String {
    let mut s = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into());
    s.push('\n');
    s
}
