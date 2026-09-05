//! Resolve an absolute repo path into an effective repo config for MCP
//! tool calls.
//!
//! If the path matches a configured repo, we use its (possibly custom)
//! CODEOWNERS path; otherwise we synthesize a virtual repo with
//! `CODEOWNERS` at the repo root — provided the path is actually a git
//! repo (`git rev-parse --show-toplevel` succeeds).

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::app_config::{AppConfigStore, RepoEntry};

use super::error::McpToolError;

#[derive(Debug, Clone)]
pub struct ResolvedRepo {
    pub abs_repo_path: PathBuf,
    /// Path to the CODEOWNERS file relative to `abs_repo_path`.
    pub codeowners_rel: String,
    /// True when this is a virtual repo (not in AppConfig).
    pub virtual_repo: bool,
}

pub fn resolve(
    store: &AppConfigStore,
    repo_arg: &str,
) -> Result<ResolvedRepo, McpToolError> {
    if repo_arg.is_empty() {
        return Err(McpToolError::InvalidArgs(
            "`repo` is required".to_string(),
        ));
    }
    let abs = Path::new(repo_arg);
    if !abs.is_absolute() {
        return Err(McpToolError::InvalidArgs(format!(
            "`repo` must be an absolute path, got {repo_arg:?}"
        )));
    }
    if !abs.is_dir() {
        return Err(McpToolError::RepoNotAGitRepo(repo_arg.to_string()));
    }

    let toplevel = git_toplevel(abs).ok_or_else(|| {
        McpToolError::RepoNotAGitRepo(repo_arg.to_string())
    })?;

    if let Some(entry) = store.find_repo_by_path(&toplevel) {
        return Ok(build_from_entry(&toplevel, &entry));
    }

    Ok(ResolvedRepo {
        abs_repo_path: toplevel,
        codeowners_rel: "CODEOWNERS".to_string(),
        virtual_repo: true,
    })
}

fn build_from_entry(toplevel: &Path, entry: &RepoEntry) -> ResolvedRepo {
    let codeowners_rel = if entry.codeowners.trim().is_empty() {
        "CODEOWNERS".to_string()
    } else {
        entry.codeowners.trim_start_matches('/').to_string()
    };
    ResolvedRepo {
        abs_repo_path: toplevel.to_path_buf(),
        codeowners_rel,
        virtual_repo: false,
    }
}

fn git_toplevel(abs: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .current_dir(abs)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(PathBuf::from(text))
    }
}
