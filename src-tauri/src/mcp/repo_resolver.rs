//! Resolve an absolute repo path into an effective repo config for MCP
//! tool calls.
//!
//! If the path matches a configured repo with a custom CODEOWNERS path,
//! we use it; otherwise we look in GitHub's locations (see
//! [`default_codeowners_rel`]) — provided the path is actually a git repo
//! (`git rev-parse --show-toplevel` succeeds).

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
        codeowners_rel: default_codeowners_rel(&toplevel),
        abs_repo_path: toplevel,
        virtual_repo: true,
    })
}

/// GitHub's lookup order: the first file that exists wins.
const DEFAULT_LOCATIONS: [&str; 3] = [".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"];

/// shortcut: checks the working tree, not the requested branch; upgrade to
/// `git cat-file -e <branch>:<path>` if branches move CODEOWNERS around.
pub fn default_codeowners_rel(toplevel: &Path) -> String {
    DEFAULT_LOCATIONS
        .iter()
        .find(|rel| toplevel.join(rel).is_file())
        .unwrap_or(&"CODEOWNERS")
        .to_string()
}

fn build_from_entry(toplevel: &Path, entry: &RepoEntry) -> ResolvedRepo {
    // The UI stores "CODEOWNERS" for every repo, so treat it as "default".
    let custom = entry.codeowners.trim().trim_start_matches('/');
    let codeowners_rel = if custom.is_empty() || custom == "CODEOWNERS" {
        default_codeowners_rel(toplevel)
    } else {
        custom.to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn repo_with(files: &[&str]) -> TempDir {
        let dir = TempDir::new().unwrap();
        let ok = Command::new("git")
            .current_dir(dir.path())
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success();
        assert!(ok);
        for f in files {
            let p = dir.path().join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "* @a\n").unwrap();
        }
        dir
    }

    fn resolve_rel(dir: &TempDir) -> String {
        let store = AppConfigStore::from_dir(TempDir::new().unwrap().path()).unwrap();
        resolve(&store, dir.path().to_str().unwrap()).unwrap().codeowners_rel
    }

    #[test]
    fn finds_codeowners_in_github_dir() {
        assert_eq!(resolve_rel(&repo_with(&[".github/CODEOWNERS"])), ".github/CODEOWNERS");
        assert_eq!(resolve_rel(&repo_with(&["docs/CODEOWNERS"])), "docs/CODEOWNERS");
    }

    #[test]
    fn github_dir_wins_over_root_like_on_github() {
        let dir = repo_with(&["CODEOWNERS", ".github/CODEOWNERS"]);
        assert_eq!(resolve_rel(&dir), ".github/CODEOWNERS");
    }

    #[test]
    fn falls_back_to_root_when_none_exist() {
        assert_eq!(resolve_rel(&repo_with(&[])), "CODEOWNERS");
    }

    #[test]
    fn custom_configured_path_is_kept() {
        let dir = repo_with(&[".github/CODEOWNERS"]);
        let entry = RepoEntry {
            id: String::new(),
            repo_path: String::new(),
            codeowners: "/teams/OWNERS".into(),
        };
        assert_eq!(build_from_entry(dir.path(), &entry).codeowners_rel, "teams/OWNERS");
        let default_entry = RepoEntry { codeowners: "CODEOWNERS".into(), ..entry };
        assert_eq!(build_from_entry(dir.path(), &default_entry).codeowners_rel, ".github/CODEOWNERS");
    }
}
