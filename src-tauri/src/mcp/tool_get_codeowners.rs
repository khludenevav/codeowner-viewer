//! Implementation of the sole MCP tool exposed by codeowners-viewer:
//! `get_codeowners`.

use std::{collections::BTreeMap, sync::Arc};

use crate::{
    app_config::AppConfigStore,
    codeowners_engine,
    codeowners_file_parser,
};

use super::{
    compactor::compact_response,
    error::McpToolError,
    path_expander::expand_paths,
    repo_resolver,
    schema::{
        CompactEntry, ForScope, FullEntry, GetCodeownersInput,
        GetCodeownersOutput, NormalEntry, ResponseMode,
    },
};

pub const HEAD_REF: &str = "HEAD";

pub fn run(
    store: &Arc<AppConfigStore>,
    input: GetCodeownersInput,
) -> Result<GetCodeownersOutput, McpToolError> {
    let repo = repo_resolver::resolve(store, &input.repo)?;
    let repo_path_str = repo
        .abs_repo_path
        .to_str()
        .ok_or_else(|| McpToolError::Internal("repo path is not UTF-8".into()))?;

    let branch = input
        .branch
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(HEAD_REF)
        .to_string();

    // ---- 1. Collect the "reachable" universe and the base expansion set.
    let (reachable, base_expansion, codeowners_content) = match input.for_scope {
        ForScope::Branch => {
            let files =
                codeowners_engine::get_branch_files_vector(repo_path_str, &branch);
            if files.is_empty() {
                // Distinguish "empty repo" vs "bad ref". If the ref
                // doesn't exist git prints nothing on stdout too, so
                // probe explicitly.
                if !rev_exists(repo_path_str, &branch) {
                    return Err(McpToolError::BranchNotFound(branch));
                }
            }
            let content = codeowners_engine::get_codeowners_content_at_ref(
                repo_path_str,
                &branch,
                &repo.codeowners_rel,
            );
            if content.is_empty() {
                return Err(McpToolError::CodeownersMissing(
                    repo.codeowners_rel.clone(),
                ));
            }
            (files.clone(), files, content)
        }
        ForScope::ChangedFiles => {
            let changed =
                codeowners_engine::get_working_tree_changed_files(repo_path_str);
            let branch_files_for_paths =
                codeowners_engine::get_branch_files_vector(repo_path_str, &branch);
            let content = codeowners_engine::get_codeowners_content_working_tree(
                repo_path_str,
                &repo.codeowners_rel,
            );
            if content.is_empty() {
                return Err(McpToolError::CodeownersMissing(
                    repo.codeowners_rel.clone(),
                ));
            }
            // Reachable universe = branch tree ∪ changed files (untracked
            // additions live in the changed set but not in the branch
            // tree yet).
            let mut reachable = branch_files_for_paths.clone();
            for f in &changed {
                if !reachable.contains(f) {
                    reachable.push(f.clone());
                }
            }
            reachable.sort();
            reachable.dedup();
            (reachable, changed, content)
        }
    };

    // ---- 2. Expand the user-provided paths against the reachable set.
    let user_paths = input.paths.clone().unwrap_or_default();
    let expanded_additional = expand_paths(
        &user_paths,
        &reachable,
        matches!(input.for_scope, ForScope::Branch),
    )?;

    let expanded_files: Vec<String> = match input.for_scope {
        ForScope::Branch => expanded_additional,
        ForScope::ChangedFiles => {
            let mut all: Vec<String> = base_expansion;
            all.extend(expanded_additional);
            all.sort();
            all.dedup();
            if all.len() > super::path_expander::MAX_EXPANDED_PATHS {
                return Err(McpToolError::TooManyPaths(
                    all.len(),
                    super::path_expander::MAX_EXPANDED_PATHS,
                ));
            }
            all
        }
    };

    // ---- 3. Resolve owners for the expanded set.
    let codeowners = codeowners_file_parser::from_reader(codeowners_content.as_bytes());
    let resolved =
        codeowners_engine::resolve_owners_batch(&codeowners, &expanded_files);

    // ---- 4. Shape the response.
    let out = match input.response_mode {
        ResponseMode::Compact => {
            let entries: Vec<CompactEntry> =
                compact_response(&expanded_files, &resolved, &reachable);
            GetCodeownersOutput::Compact(entries)
        }
        ResponseMode::Normal => {
            let mut map: BTreeMap<String, NormalEntry> = BTreeMap::new();
            for (path, res) in expanded_files.iter().zip(resolved.iter()) {
                map.insert(
                    path.clone(),
                    NormalEntry {
                        owners: res.owners.clone(),
                        rule_comment: res.rule_comment.clone(),
                    },
                );
            }
            GetCodeownersOutput::Normal(map)
        }
        ResponseMode::Full => {
            let mut map: BTreeMap<String, FullEntry> = BTreeMap::new();
            for (path, res) in expanded_files.iter().zip(resolved.iter()) {
                let line = if res.rule_line_number == 0 {
                    -1
                } else {
                    res.rule_line_number as i64
                };
                map.insert(
                    path.clone(),
                    FullEntry {
                        owners: res.owners.clone(),
                        rule_comment: res.rule_comment.clone(),
                        rule_line_number: line,
                    },
                );
            }
            GetCodeownersOutput::Full(map)
        }
    };

    Ok(out)
}

fn rev_exists(abs_repo_path: &str, rev: &str) -> bool {
    let output = std::process::Command::new("git")
        .current_dir(abs_repo_path)
        .arg("rev-parse")
        .arg("--verify")
        .arg("--quiet")
        .arg(rev)
        .output();
    matches!(output, Ok(o) if o.status.success())
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::sync::Arc;

    use tempfile::TempDir;

    use crate::app_config::AppConfigStore;
    use crate::mcp::schema::{ForScope, ResponseMode};

    use super::*;

    struct RepoFixture {
        _dir: TempDir,
        path: String,
    }

    fn make_repo() -> RepoFixture {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        run_git(&path, &["init", "-q", "-b", "main"]);
        run_git(&path, &["config", "user.email", "test@example.com"]);
        run_git(&path, &["config", "user.name", "Test"]);
        std::fs::write(
            path.join("CODEOWNERS"),
            "* @global\n\
             /a/** @team-a\n\
             /b/*.rs @team-b #!required\n",
        )
        .unwrap();
        std::fs::create_dir_all(path.join("a")).unwrap();
        std::fs::write(path.join("a/one.rs"), "").unwrap();
        std::fs::write(path.join("a/two.rs"), "").unwrap();
        std::fs::create_dir_all(path.join("b")).unwrap();
        std::fs::write(path.join("b/c.rs"), "").unwrap();
        std::fs::write(path.join("README.md"), "").unwrap();
        run_git(&path, &["add", "-A"]);
        run_git(&path, &["commit", "-q", "-m", "init"]);
        let path_str = path.to_string_lossy().into_owned();
        RepoFixture { _dir: dir, path: path_str }
    }

    fn run_git(dir: &std::path::Path, args: &[&str]) {
        let out = Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn empty_store() -> Arc<AppConfigStore> {
        let tmp = TempDir::new().unwrap();
        Arc::new(AppConfigStore::from_dir(tmp.path()).unwrap())
    }

    #[test]
    fn branch_mode_normal_returns_owners_per_file() {
        let repo = make_repo();
        let store = empty_store();
        let out = run(
            &store,
            GetCodeownersInput {
                repo: repo.path.clone(),
                for_scope: ForScope::Branch,
                branch: None,
                paths: Some(vec!["a".into()]),
                response_mode: ResponseMode::Normal,
            },
        )
        .unwrap();
        match out {
            GetCodeownersOutput::Normal(map) => {
                assert_eq!(map.get("a/one.rs").unwrap().owners, "@team-a");
                assert_eq!(map.get("a/two.rs").unwrap().owners, "@team-a");
            }
            _ => panic!("expected Normal"),
        }
    }

    #[test]
    fn branch_mode_compact_collapses_uniform_dir() {
        let repo = make_repo();
        let store = empty_store();
        let out = run(
            &store,
            GetCodeownersInput {
                repo: repo.path.clone(),
                for_scope: ForScope::Branch,
                branch: None,
                paths: Some(vec!["a".into()]),
                response_mode: ResponseMode::Compact,
            },
        )
        .unwrap();
        match out {
            GetCodeownersOutput::Compact(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].owners, "@team-a");
                assert_eq!(entries[0].paths, vec!["a".to_string()]);
            }
            _ => panic!("expected Compact"),
        }
    }

    #[test]
    fn full_mode_returns_line_numbers() {
        let repo = make_repo();
        let store = empty_store();
        let out = run(
            &store,
            GetCodeownersInput {
                repo: repo.path.clone(),
                for_scope: ForScope::Branch,
                branch: Some("HEAD".into()),
                paths: Some(vec!["b/c.rs".into()]),
                response_mode: ResponseMode::Full,
            },
        )
        .unwrap();
        match out {
            GetCodeownersOutput::Full(map) => {
                let entry = map.get("b/c.rs").unwrap();
                assert_eq!(entry.owners, "@team-b");
                assert_eq!(entry.rule_comment.as_deref(), Some("#!required"));
                assert!(entry.rule_line_number > 0);
            }
            _ => panic!("expected Full"),
        }
    }

    #[test]
    fn changed_files_mode_returns_working_tree_changes() {
        let repo = make_repo();
        let store = empty_store();
        // Modify one file and add an untracked one.
        std::fs::write(
            std::path::Path::new(&repo.path).join("a/one.rs"),
            "x",
        )
        .unwrap();
        std::fs::write(
            std::path::Path::new(&repo.path).join("new.rs"),
            "",
        )
        .unwrap();
        let out = run(
            &store,
            GetCodeownersInput {
                repo: repo.path.clone(),
                for_scope: ForScope::ChangedFiles,
                branch: None,
                paths: None,
                response_mode: ResponseMode::Normal,
            },
        )
        .unwrap();
        match out {
            GetCodeownersOutput::Normal(map) => {
                assert!(map.contains_key("a/one.rs"));
                assert!(map.contains_key("new.rs"));
                assert_eq!(map.get("new.rs").unwrap().owners, "@global");
            }
            _ => panic!("expected Normal"),
        }
    }

    #[test]
    fn repo_not_a_git_repo_returns_error() {
        let tmp = TempDir::new().unwrap();
        let store = empty_store();
        let err = run(
            &store,
            GetCodeownersInput {
                repo: tmp.path().to_string_lossy().into_owned(),
                for_scope: ForScope::Branch,
                branch: None,
                paths: None,
                response_mode: ResponseMode::Normal,
            },
        )
        .unwrap_err();
        matches!(err, McpToolError::RepoNotAGitRepo(_));
    }

    #[test]
    fn absolute_path_in_paths_is_rejected() {
        let repo = make_repo();
        let store = empty_store();
        let err = run(
            &store,
            GetCodeownersInput {
                repo: repo.path.clone(),
                for_scope: ForScope::Branch,
                branch: None,
                paths: Some(vec!["/etc/passwd".into()]),
                response_mode: ResponseMode::Normal,
            },
        )
        .unwrap_err();
        matches!(err, McpToolError::PathOutsideRepo(_));
    }
}
