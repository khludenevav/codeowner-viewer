//! Implementation of the MCP `export_codeowners` tool: dumps the full
//! codeowners map for a repo to a JSON file under the OS temp dir and
//! returns just its path + size + counts. See `export_builder` for the
//! schema and filter semantics.
//!
//! The dump is intended to be consumed by short throwaway scripts (e.g.
//! Python) that use the JSON stdlib directly. It's NOT meant to be
//! read inline by the agent — for that, `get_codeowners` is a better
//! fit.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use chrono::{Duration, Utc};

use crate::app_config::AppConfigStore;

use super::{
    error::McpToolError,
    export_builder::{self, ExportFilters, ExportPayload, SCHEMA_VERSION},
    repo_resolver,
    schema::{ExportCodeownersInput, ExportCodeownersOutput},
    tool_get_codeowners::HEAD_REF,
};

/// Sub-directory of `env::temp_dir()` where dumps live.
pub const EXPORT_DIR_NAME: &str = "codeowners-viewer";
/// Files matching `export-*.json` older than this are pruned on every
/// invocation.
pub const RETENTION_DAYS: i64 = 7;

pub fn run(
    store: &Arc<AppConfigStore>,
    input: ExportCodeownersInput,
) -> Result<ExportCodeownersOutput, McpToolError> {
    let repo = repo_resolver::resolve(store, &input.repo)?;

    let filters = ExportFilters {
        owners: input.owners,
        extensions: input.extensions,
    };

    let payload = export_builder::build_export_payload(&repo, HEAD_REF, &filters)?;

    let path = match input.path.as_deref() {
        Some(custom) => write_payload_to(custom, &payload)?,
        None => {
            let dir = ensure_export_dir()?;
            let _ = prune_old(&dir);
            write_payload(&dir, &payload)?
        }
    };
    let size_bytes = fs::metadata(&path)
        .map(|m| m.len())
        .map_err(|e| McpToolError::Internal(format!("failed to stat dump: {e}")))?;

    Ok(ExportCodeownersOutput {
        path: path.to_string_lossy().into_owned(),
        size_bytes,
        file_count: payload.stats.total_files,
        rule_count: payload.stats.rule_count,
        schema: SCHEMA_VERSION.to_string(),
    })
}

fn ensure_export_dir() -> Result<PathBuf, McpToolError> {
    let dir = std::env::temp_dir().join(EXPORT_DIR_NAME);
    fs::create_dir_all(&dir)
        .map_err(|e| McpToolError::Internal(format!("failed to create dump dir: {e}")))?;
    Ok(dir)
}

fn write_payload(dir: &Path, payload: &ExportPayload) -> Result<PathBuf, McpToolError> {
    use rand::RngCore;
    let mut buf = [0u8; 4];
    rand::rng().fill_bytes(&mut buf);
    let suffix: String = buf.iter().map(|b| format!("{:02x}", b)).collect();
    let ts = Utc::now().format("%Y%m%dT%H%M%SZ");
    let path = dir.join(format!("export-{ts}-{suffix}.json"));

    let json = serde_json::to_string_pretty(payload)
        .map_err(|e| McpToolError::Internal(format!("failed to serialize dump: {e}")))?;
    fs::write(&path, json)
        .map_err(|e| McpToolError::Internal(format!("failed to write dump: {e}")))?;
    Ok(path)
}

/// Write to a caller-provided location. Requires an absolute path and
/// overwrites any existing file. Parent directories are created as
/// needed. The path is NOT touched by the retention prune.
fn write_payload_to(raw: &str, payload: &ExportPayload) -> Result<PathBuf, McpToolError> {
    let path = PathBuf::from(raw);
    if !path.is_absolute() {
        return Err(McpToolError::Internal(format!(
            "`path` must be absolute: {raw}"
        )));
    }
    if raw.ends_with(std::path::MAIN_SEPARATOR) || path.is_dir() {
        return Err(McpToolError::Internal(format!(
            "`path` must be a file, not a directory: {raw}"
        )));
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                McpToolError::Internal(format!(
                    "failed to create parent directory for dump: {e}"
                ))
            })?;
        }
    }
    let json = serde_json::to_string_pretty(payload)
        .map_err(|e| McpToolError::Internal(format!("failed to serialize dump: {e}")))?;
    fs::write(&path, json)
        .map_err(|e| McpToolError::Internal(format!("failed to write dump: {e}")))?;
    Ok(path)
}

/// Drop `export-*.json` files older than `RETENTION_DAYS`.
fn prune_old(dir: &Path) -> std::io::Result<()> {
    let cutoff = Utc::now() - Duration::days(RETENTION_DAYS);
    prune_older_than(dir, cutoff)
}

fn prune_older_than(dir: &Path, cutoff: chrono::DateTime<Utc>) -> std::io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if !name.starts_with("export-") || !name.ends_with(".json") {
            continue;
        }
        let modified: SystemTime = match entry.metadata().and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let dt: chrono::DateTime<Utc> = modified.into();
        if dt < cutoff {
            let _ = fs::remove_file(&path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::export_builder::{ExportPayload, RulePayload, StatsPayload};
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    fn sample_payload() -> ExportPayload {
        let mut rules: BTreeMap<String, RulePayload> = BTreeMap::new();
        rules.insert(
            "42".to_string(),
            RulePayload {
                owners: vec!["@t/a".to_string()],
                comment: None,
            },
        );
        let mut files: BTreeMap<String, u32> = BTreeMap::new();
        files.insert("app/x.java".to_string(), 42);
        ExportPayload {
            schema: SCHEMA_VERSION.to_string(),
            rules,
            files,
            stats: StatsPayload {
                total_files: 1,
                owned_files: 1,
                unowned_files: 0,
                rule_count: 1,
                codeowners_file_path: "CODEOWNERS".to_string(),
                filtered: false,
                generated_at: "2026-09-06T18:15:28Z".to_string(),
            },
        }
    }

    #[test]
    fn write_produces_readable_json() {
        let dir = TempDir::new().unwrap();
        let path = write_payload(dir.path(), &sample_payload()).unwrap();
        assert!(path.exists());
        let text = fs::read_to_string(&path).unwrap();
        let back: ExportPayload = serde_json::from_str(&text).unwrap();
        assert_eq!(back.schema, "codeowners-export/v1");
        assert_eq!(back.files["app/x.java"], 42);
        assert_eq!(back.stats.generated_at, "2026-09-06T18:15:28Z");
    }

    #[test]
    fn prune_removes_only_matching_files() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        // A fresh dump.
        let fresh = path.join("export-fresh.json");
        fs::write(&fresh, "{}").unwrap();

        // Unrelated file — must survive even the "prune everything old" pass.
        let stranger = path.join("stranger.txt");
        fs::write(&stranger, "keep me").unwrap();

        // Another matching-name file — will be pruned by future-dated cutoff.
        let target = path.join("export-target.json");
        fs::write(&target, "{}").unwrap();

        // Cutoff = tomorrow → both matching files considered "old" and dropped.
        let cutoff = Utc::now() + Duration::days(1);
        prune_older_than(path, cutoff).unwrap();

        assert!(!fresh.exists(), "matching-name fresh file should have been pruned");
        assert!(!target.exists(), "matching-name file should have been pruned");
        assert!(
            stranger.exists(),
            "non-matching file must never be pruned"
        );
    }

    #[test]
    fn prune_leaves_recent_files_alone() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        let fresh = path.join("export-fresh.json");
        fs::write(&fresh, "{}").unwrap();

        // Cutoff = yesterday → fresh file survives.
        let cutoff = Utc::now() - Duration::days(1);
        prune_older_than(path, cutoff).unwrap();

        assert!(fresh.exists());
    }

    #[test]
    fn write_to_custom_path_creates_parent_dirs_and_overwrites() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("nested").join("sub").join("my-dump.json");

        // Parent chain does not exist yet.
        assert!(!nested.parent().unwrap().exists());

        let out = write_payload_to(nested.to_str().unwrap(), &sample_payload()).unwrap();
        assert_eq!(out, nested);
        assert!(nested.exists());

        // Second write must overwrite in place without error.
        let mut second = sample_payload();
        second.stats.total_files = 99;
        write_payload_to(nested.to_str().unwrap(), &second).unwrap();

        let back: ExportPayload =
            serde_json::from_str(&fs::read_to_string(&nested).unwrap()).unwrap();
        assert_eq!(back.stats.total_files, 99);
    }

    #[test]
    fn write_to_custom_path_rejects_relative_path() {
        let err = write_payload_to("relative/path.json", &sample_payload()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("absolute"), "expected 'absolute' hint, got: {msg}");
    }

    #[test]
    fn write_to_custom_path_rejects_directory_target() {
        let dir = TempDir::new().unwrap();
        // Path with trailing separator is treated as a directory target.
        let mut raw = dir.path().to_string_lossy().into_owned();
        raw.push(std::path::MAIN_SEPARATOR);
        let err = write_payload_to(&raw, &sample_payload()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("directory"), "expected 'directory' hint, got: {msg}");
    }
}
