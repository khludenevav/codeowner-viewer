//! Implementation of the sole MCP tool exposed by codeowners-viewer:
//! `get_codeowners`.
//!
//! Response is a plain-text DSL body (see `dsl_emitter.rs`). Structure:
//!
//! ```text
//! format: codeowners-dsl-v1
//! base: <stripped-common-prefix>
//! default: <rule-id>
//! truncated: true|false
//! sizeBytes: <int>
//! fullDumpPath: <optional; present when truncated>
//!
//! rules:
//!   <line-number> <owners> [(<comment>)]
//!   ...
//!
//! <dir>/[ <rule-id>]
//!   <file>[ <rule-id>]
//!   <subdir>/ ...
//! ```

use std::sync::Arc;

use crate::{
    app_config::AppConfigStore,
    codeowners_engine,
    codeowners_file_parser,
};

use super::{
    compactor::build_annotated_tree,
    dump_store::DumpStore,
    error::McpToolError,
    path_expander::expand_paths,
    repo_resolver,
    schema::{ForScope, GetCodeownersInput},
    size_guard::{self, DEFAULT_SIZE_LIMIT},
};

pub const HEAD_REF: &str = "HEAD";

#[derive(Debug)]
pub struct RunOutput {
    pub body: String,
    pub truncated: bool,
    pub dump_path: Option<std::path::PathBuf>,
}

pub fn run(
    store: &Arc<AppConfigStore>,
    dump_store: Option<&Arc<DumpStore>>,
    input: GetCodeownersInput,
) -> Result<RunOutput, McpToolError> {
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
            if files.is_empty() && !rev_exists(repo_path_str, &branch) {
                return Err(McpToolError::BranchNotFound(branch));
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

    // ---- 4. Build the annotated tree.
    let user_paths_for_depth: Vec<String> = input.paths.clone().unwrap_or_default();
    let mut tree = build_annotated_tree(
        &expanded_files,
        &resolved,
        input.max_depth,
        &user_paths_for_depth,
    );

    // ---- 5. Pick the dump path up front: the size guard writes it into
    //         the header, and the file is written to that same path.
    let dump_path = dump_store.map(|s| s.new_path());

    let guarded = size_guard::apply(
        &mut tree,
        input.response_mode,
        DEFAULT_SIZE_LIMIT,
        dump_path.as_deref().and_then(|p| p.to_str()),
    );

    let dump_path = match (guarded.truncated, dump_path, guarded.full_body.as_deref()) {
        (true, Some(path), Some(full)) => std::fs::write(&path, full).ok().map(|_| path),
        _ => None,
    };

    Ok(RunOutput { body: guarded.body, truncated: guarded.truncated, dump_path })
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
    fn truncated_response_points_to_existing_dump() {
        let repo = make_repo();
        let root = std::path::Path::new(&repo.path);
        let dir = root.join("big");
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..2000 {
            std::fs::write(dir.join(format!("file_with_a_rather_long_name_{i:04}.txt")), "").unwrap();
        }
        run_git(root, &["add", "-A"]);
        run_git(root, &["commit", "-q", "-m", "big"]);
        let dumps = TempDir::new().unwrap();
        let dump_store = Arc::new(DumpStore::open(dumps.path()));
        let out = run(
            &empty_store(),
            Some(&dump_store),
            GetCodeownersInput {
                repo: repo.path.clone(),
                for_scope: ForScope::Branch,
                branch: None,
                paths: Some(vec!["big".into()]),
                response_mode: ResponseMode::Full,
                max_depth: None,
            },
        )
        .unwrap();
        assert!(out.truncated);
        let header_path = out
            .body
            .lines()
            .find_map(|l| l.strip_prefix("fullDumpPath: "))
            .expect("header has fullDumpPath");
        assert!(std::path::Path::new(header_path).is_file(), "{header_path} missing");
        assert_eq!(out.dump_path.as_deref(), Some(std::path::Path::new(header_path)));
    }

    #[test]
    fn branch_mode_full_lists_every_file_with_rule_id() {
        let repo = make_repo();
        let store = empty_store();
        let out = run(
            &store,
            None,
            GetCodeownersInput {
                repo: repo.path.clone(),
                for_scope: ForScope::Branch,
                branch: None,
                paths: Some(vec!["a".into()]),
                response_mode: ResponseMode::Full,
                max_depth: None,
            },
        )
        .unwrap();
        assert!(out.body.contains("format: codeowners-dsl-v1"));
        assert!(out.body.contains("base: a"));
        assert!(out.body.contains("one.rs"));
        assert!(out.body.contains("two.rs"));
        assert!(!out.truncated);
    }

    #[test]
    fn directory_rule_without_trailing_slash_owns_nested_files() {
        let repo = make_repo();
        let path = std::path::Path::new(&repo.path);
        std::fs::write(
            path.join("CODEOWNERS"),
            "/worker/connector_generator @team-gen\n",
        )
        .unwrap();
        std::fs::create_dir_all(path.join("worker/connector_generator/src")).unwrap();
        std::fs::write(path.join("worker/connector_generator/Dockerfile"), "").unwrap();
        std::fs::write(path.join("worker/connector_generator/src/Main.java"), "").unwrap();
        run_git(path, &["add", "-A"]);
        run_git(path, &["commit", "-q", "-m", "dir rule"]);

        let store = empty_store();
        let out = run(
            &store,
            None,
            GetCodeownersInput {
                repo: repo.path.clone(),
                for_scope: ForScope::Branch,
                branch: None,
                paths: Some(vec![
                    "worker/connector_generator/Dockerfile".into(),
                    "worker/connector_generator/src/Main.java".into(),
                ]),
                response_mode: ResponseMode::Full,
                max_depth: None,
            },
        )
        .unwrap();
        let lines: Vec<&str> = out.body.lines().collect();
        assert!(lines.contains(&" 1 @team-gen"), "{}", out.body);
        assert!(lines.contains(&"Dockerfile 1"), "{}", out.body);
        assert!(lines.contains(&" Main.java 1"), "{}", out.body);
        assert!(!out.body.contains("(unowned)"), "{}", out.body);
    }

    #[test]
    fn branch_mode_compact_omits_files_matching_default() {
        let repo = make_repo();
        let store = empty_store();
        let out = run(
            &store,
            None,
            GetCodeownersInput {
                repo: repo.path.clone(),
                for_scope: ForScope::Branch,
                branch: None,
                paths: Some(vec!["a".into()]),
                response_mode: ResponseMode::Compact,
                max_depth: None,
            },
        )
        .unwrap();
        // All files in "a" match the same rule => nothing but the header
        // (plus rules table and possibly the base directory line).
        assert!(!out.body.contains("one.rs"));
        assert!(!out.body.contains("two.rs"));
    }

    #[test]
    fn full_mode_includes_rules_table_with_comment() {
        let repo = make_repo();
        let store = empty_store();
        let out = run(
            &store,
            None,
            GetCodeownersInput {
                repo: repo.path.clone(),
                for_scope: ForScope::Branch,
                branch: Some("HEAD".into()),
                paths: Some(vec!["b/c.rs".into()]),
                response_mode: ResponseMode::Full,
                max_depth: None,
            },
        )
        .unwrap();
        assert!(out.body.contains("@team-b"));
        assert!(out.body.contains("(!required)"));
    }

    #[test]
    fn changed_files_mode_returns_working_tree_changes() {
        let repo = make_repo();
        let store = empty_store();
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
            None,
            GetCodeownersInput {
                repo: repo.path.clone(),
                for_scope: ForScope::ChangedFiles,
                branch: None,
                paths: None,
                response_mode: ResponseMode::Full,
                max_depth: None,
            },
        )
        .unwrap();
        assert!(out.body.contains("one.rs"));
        assert!(out.body.contains("new.rs"));
    }

    #[test]
    fn repo_not_a_git_repo_returns_error() {
        let tmp = TempDir::new().unwrap();
        let store = empty_store();
        let err = run(
            &store,
            None,
            GetCodeownersInput {
                repo: tmp.path().to_string_lossy().into_owned(),
                for_scope: ForScope::Branch,
                branch: None,
                paths: None,
                response_mode: ResponseMode::Full,
                max_depth: None,
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
            None,
            GetCodeownersInput {
                repo: repo.path.clone(),
                for_scope: ForScope::Branch,
                branch: None,
                paths: Some(vec!["/etc/passwd".into()]),
                response_mode: ResponseMode::Full,
                max_depth: None,
            },
        )
        .unwrap_err();
        matches!(err, McpToolError::PathOutsideRepo(_));
    }

    fn make_repo_with_depth() -> RepoFixture {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        run_git(&path, &["init", "-q", "-b", "main"]);
        run_git(&path, &["config", "user.email", "test@example.com"]);
        run_git(&path, &["config", "user.name", "Test"]);
        // Mixed subtree "backstage" (api → team-a, db → team-b);
        // uniform subtree "core" (all team-c). Second top-level dir
        // "docs" prevents the base prefix from collapsing everything.
        std::fs::write(
            path.join("CODEOWNERS"),
            "/backstage/api/** @team-a\n\
             /backstage/db/** @team-b\n\
             /core/** @team-c\n\
             /docs/** @docs\n",
        )
        .unwrap();
        std::fs::create_dir_all(path.join("backstage/api")).unwrap();
        std::fs::create_dir_all(path.join("backstage/db")).unwrap();
        std::fs::create_dir_all(path.join("core/util")).unwrap();
        std::fs::create_dir_all(path.join("docs")).unwrap();
        std::fs::write(path.join("backstage/api/one.rs"), "").unwrap();
        std::fs::write(path.join("backstage/api/two.rs"), "").unwrap();
        std::fs::write(path.join("backstage/db/three.rs"), "").unwrap();
        std::fs::write(path.join("core/util/x.rs"), "").unwrap();
        std::fs::write(path.join("core/util/y.rs"), "").unwrap();
        std::fs::write(path.join("docs/readme.md"), "").unwrap();
        run_git(&path, &["add", "-A"]);
        run_git(&path, &["commit", "-q", "-m", "init"]);
        let path_str = path.to_string_lossy().into_owned();
        RepoFixture { _dir: dir, path: path_str }
    }

    #[test]
    fn max_depth_truncates_non_uniform_subtree() {
        let repo = make_repo_with_depth();
        let store = empty_store();
        // paths=["backstage"] scopes the expanded set to just backstage
        // files, so base="backstage" and the anchor lands on the root.
        // maxDepth=0 collapses each root child (api, db) individually.
        // Both are uniform so they render as plain `dir/ <rule>` with
        // no file lines beneath.
        let out = run(
            &store,
            None,
            GetCodeownersInput {
                repo: repo.path.clone(),
                for_scope: ForScope::Branch,
                branch: None,
                paths: Some(vec!["backstage".into()]),
                response_mode: ResponseMode::Full,
                max_depth: Some(0),
            },
        )
        .unwrap();
        assert!(!out.body.contains(".rs"), "no file lines past the depth budget\n{}", out.body);
    }

    #[test]
    fn max_depth_marks_mixed_frontier_as_truncated() {
        let repo = make_repo_with_depth();
        let store = empty_store();
        // Requesting two disjoint top-level paths keeps the base empty,
        // so anchors become concrete "backstage" / "docs" nodes.
        // maxDepth=0 collapses each anchor: backstage is mixed →
        // TRUNCATED marker with per-rule counts; docs is uniform.
        let out = run(
            &store,
            None,
            GetCodeownersInput {
                repo: repo.path.clone(),
                for_scope: ForScope::Branch,
                branch: None,
                paths: Some(vec!["backstage".into(), "docs".into()]),
                response_mode: ResponseMode::Compact,
                max_depth: Some(0),
            },
        )
        .unwrap();
        assert!(
            out.body.contains("TRUNCATED"),
            "backstage should be TRUNCATED, body was:\n{}",
            out.body
        );
        let truncated_line = out
            .body
            .lines()
            .find(|l| l.contains("TRUNCATED"))
            .unwrap_or("");
        assert!(
            truncated_line.contains(':'),
            "TRUNCATED line should carry id:count, got: {truncated_line}"
        );
    }
}
