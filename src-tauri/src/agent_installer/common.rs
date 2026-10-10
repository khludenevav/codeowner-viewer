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

/// Read a JSON config. Missing or blank file → `{}`. A file that does
/// not parse (JSONC comments, a half-written file) is an error: writing
/// back `{}` + our entry would wipe the user's config.
pub fn read_json(file: &Path) -> anyhow::Result<serde_json::Value> {
    let text = read_or_blank(file)?;
    if text.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("cannot parse {}: {e}", file.display()))?;
    if !value.is_object() {
        anyhow::bail!("{} is not a JSON object", file.display());
    }
    Ok(value)
}

/// TOML twin of [`read_json`].
pub fn read_toml(file: &Path) -> anyhow::Result<toml_edit::DocumentMut> {
    read_or_blank(file)?
        .parse()
        .map_err(|e| anyhow::anyhow!("cannot parse {}: {e}", file.display()))
}

fn read_or_blank(file: &Path) -> anyhow::Result<String> {
    match fs::read_to_string(file) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e.into()),
    }
}

/// Format a `Value` back into pretty JSON, appending a trailing newline
/// (matching common editor conventions).
pub fn pretty_json(value: &serde_json::Value) -> String {
    let mut s = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into());
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_json_missing_or_blank_is_empty_object() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("c.json");
        assert_eq!(read_json(&file).unwrap(), serde_json::json!({}));
        fs::write(&file, "  \n").unwrap();
        assert_eq!(read_json(&file).unwrap(), serde_json::json!({}));
    }

    #[test]
    fn read_json_rejects_unparseable_and_non_object() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("c.json");
        fs::write(&file, "{ // comment\n \"servers\": {} }").unwrap();
        assert!(read_json(&file).is_err());
        fs::write(&file, "[]").unwrap();
        assert!(read_json(&file).is_err());
    }

    #[test]
    fn read_toml_rejects_unparseable() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("c.toml");
        assert!(read_toml(&file).unwrap().is_empty());
        fs::write(&file, "[mcp_servers\nurl = ").unwrap();
        assert!(read_toml(&file).is_err());
    }
}
