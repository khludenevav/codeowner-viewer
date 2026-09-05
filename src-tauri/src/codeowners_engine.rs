//! Shared, transport-agnostic codeowners engine.
//!
//! Both the Tauri commands in [`crate::main`] and the MCP `get_codeowners`
//! tool (see [`crate::mcp`]) go through this module so we only have one
//! implementation of "which git files exist" and "what does the ancestor
//! cache look like".

use std::{fs, path::Path, process::Command};

use ahash::AHashMap;
use rayon::prelude::*;

use crate::codeowners_file_parser::{self, Owners};

/// Sentinel used by the MCP `for = "branch"` mode to mean "resolve against
/// the working tree's currently checked out commit".
pub const HEAD_REF: &str = "HEAD";

/// Structured info about a single rule that matched a file. Used by the
/// MCP tool to build normal/full response modes.
#[derive(Debug, Clone)]
pub struct OwnerResolution {
    pub owners: String,
    pub rule_comment: Option<String>,
    pub rule_line_number: u32,
}

impl OwnerResolution {
    pub fn unowned() -> Self {
        Self { owners: String::new(), rule_comment: None, rule_line_number: 0 }
    }
}

/// Enumerate every file present in the repo at `branch`. `branch` may be
/// `HEAD`, a named ref, or any revision `git ls-tree` accepts.
pub fn get_branch_files_vector(abs_repo_path: &str, branch: &str) -> Vec<String> {
    let output = Command::new("git")
        .current_dir(abs_repo_path)
        .arg("ls-tree")
        .arg("-r")
        .arg(branch)
        .arg("--name-only")
        .output()
        .expect("git command failed");

    if !output.status.success() {
        eprintln!("git ls-tree error: {}", String::from_utf8_lossy(&output.stderr));
    }

    let mut branch_files: Vec<String> = Vec::new();
    for file_path in String::from_utf8_lossy(&output.stdout).split('\n') {
        if !file_path.is_empty() {
            branch_files.push(file_path.to_string());
        }
    }
    branch_files
}

/// Files changed between `branch` and `origin/main`. Same behavior as the
/// existing branch-changes UI.
pub fn get_branch_diff(abs_repo_path: &str, branch: &str) -> String {
    let output = Command::new("git")
        .current_dir(abs_repo_path)
        .arg("--no-pager")
        .arg("diff")
        .arg("--name-only")
        .arg(format!("origin/main...{branch}"))
        .output()
        .expect("git command failed");
    if !output.status.success() {
        eprintln!("git diff error: {}", String::from_utf8_lossy(&output.stderr));
    }
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Working-tree changed files: staged + unstaged + untracked (equivalent
/// to `git status --porcelain=v1 -uall`). Repo-relative paths.
pub fn get_working_tree_changed_files(abs_repo_path: &str) -> Vec<String> {
    let output = Command::new("git")
        .current_dir(abs_repo_path)
        .arg("--no-pager")
        .arg("status")
        .arg("--porcelain=v1")
        .arg("-uall")
        .output()
        .expect("git command failed");
    if !output.status.success() {
        eprintln!("git status error: {}", String::from_utf8_lossy(&output.stderr));
    }
    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    let mut out = Vec::new();
    for line in raw.lines() {
        // porcelain v1 lines are "XY path" or "XY orig -> renamed"
        if line.len() < 4 {
            continue;
        }
        let rest = &line[3..];
        let path = match rest.rsplit_once(" -> ") {
            Some((_, new_name)) => new_name,
            None => rest,
        };
        let path = path.trim().trim_matches('"');
        if !path.is_empty() {
            out.push(path.to_string());
        }
    }
    out
}

/// Read the CODEOWNERS file at `codeowners_rel` (repo-relative) as it
/// existed at `branch`. Falls back to an empty string on error so the
/// downstream parser can produce an empty `Owners` — matches the existing
/// behavior in `main.rs`.
pub fn get_codeowners_content_at_ref(
    abs_repo_path: &str,
    branch: &str,
    codeowners_rel: &str,
) -> String {
    let output = Command::new("git")
        .current_dir(abs_repo_path)
        .arg("--no-pager")
        .arg("show")
        .arg(format!("{branch}:{codeowners_rel}"))
        .output()
        .expect("git command failed");
    if !output.status.success() {
        eprintln!(
            "git show {branch}:{codeowners_rel} error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Read the CODEOWNERS file straight from the working tree. Used by the
/// MCP `for = "changed_files"` mode so local edits to CODEOWNERS are
/// respected.
pub fn get_codeowners_content_working_tree(
    abs_repo_path: &str,
    codeowners_rel: &str,
) -> String {
    let full = Path::new(abs_repo_path).join(codeowners_rel);
    fs::read_to_string(&full).unwrap_or_default()
}

/// Precompute `ancestor_first_i(dir)` for every unique parent directory
/// in `files`. Identical algorithm to the one previously living inline
/// in `main.rs` — kept here so the MCP tool can reuse it.
pub fn build_ancestor_cache(
    codeowners: &Owners,
    files: &[String],
) -> AHashMap<String, Option<usize>> {
    let mut unique_dirs: AHashMap<String, ()> = AHashMap::new();
    for file in files {
        let path = Path::new(file);
        let mut cur = path.parent();
        while let Some(dir) = cur {
            let key = dir.to_str().unwrap_or("");
            if unique_dirs.insert(key.to_string(), ()).is_some() {
                break;
            }
            cur = dir.parent();
        }
    }
    let dirs: Vec<String> = unique_dirs.into_keys().collect();

    let direct: Vec<(String, Option<usize>)> = dirs
        .par_iter()
        .map(|d| {
            let idx = codeowners.direct_match_index_at(Path::new(d));
            (d.clone(), idx)
        })
        .collect();
    let mut direct_map: AHashMap<String, Option<usize>> =
        AHashMap::with_capacity(direct.len());
    for (d, i) in direct {
        direct_map.insert(d, i);
    }

    let mut cache: AHashMap<String, Option<usize>> =
        AHashMap::with_capacity(direct_map.len());
    let keys: Vec<String> = direct_map.keys().cloned().collect();
    for k in keys {
        combine_ancestor_index(&direct_map, &mut cache, &k);
    }
    cache
}

fn combine_ancestor_index(
    direct: &AHashMap<String, Option<usize>>,
    cache: &mut AHashMap<String, Option<usize>>,
    dir_key: &str,
) -> Option<usize> {
    if let Some(v) = cache.get(dir_key) {
        return *v;
    }
    let self_match = direct.get(dir_key).copied().flatten();
    let parent_match = match Path::new(dir_key).parent() {
        Some(p) => {
            let pk = p.to_str().unwrap_or("");
            if pk.is_empty() && dir_key.is_empty() {
                None
            } else if direct.contains_key(pk) {
                combine_ancestor_index(direct, cache, pk)
            } else {
                None
            }
        }
        None => None,
    };
    let result = match (self_match, parent_match) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    cache.insert(dir_key.to_string(), result);
    result
}

pub fn ancestor_index_for(
    cache: &AHashMap<String, Option<usize>>,
    file_path: &Path,
) -> Option<usize> {
    let parent = file_path.parent()?;
    let key = parent.to_str().unwrap_or("");
    cache.get(key).copied().flatten()
}

/// Batch-resolve owners for `files`. Uses the multithreaded ancestor
/// cache the app already ships. Returns one `OwnerResolution` per input
/// file, in the same order as `files`.
///
/// Unowned files get `OwnerResolution::unowned()` (empty owners string,
/// no comment, line 0). The caller decides how to render that (the MCP
/// tool maps line 0 to `-1` in full response mode).
pub fn resolve_owners_batch(
    codeowners: &Owners,
    files: &[String],
) -> Vec<OwnerResolution> {
    let owner_strings = codeowners.owner_strings();

    // The ancestor cache is only worth building when we have many files
    // to resolve; for small requests the O(rules × files) direct
    // resolution is fine.
    if files.len() < 32 {
        return files
            .iter()
            .map(|f| {
                let path = Path::new(f);
                let idx = codeowners.of_index_with_ancestor(path, None);
                to_resolution(codeowners, &owner_strings, idx)
            })
            .collect();
    }

    let cache = build_ancestor_cache(codeowners, files);
    let num_threads = rayon::current_num_threads().max(1);
    let chunk_size = 512usize.max(files.len() / (num_threads * 8).max(1));
    files
        .par_chunks(chunk_size)
        .flat_map_iter(|chunk| {
            let out: Vec<OwnerResolution> = chunk
                .iter()
                .map(|file| {
                    let path = Path::new(file);
                    let ancestor = ancestor_index_for(&cache, path);
                    let idx = codeowners.of_index_with_ancestor(path, ancestor);
                    to_resolution(codeowners, &owner_strings, idx)
                })
                .collect();
            out.into_iter()
        })
        .collect()
}

fn to_resolution(
    codeowners: &Owners,
    owner_strings: &[String],
    idx: Option<usize>,
) -> OwnerResolution {
    match idx {
        Some(i) => OwnerResolution {
            owners: owner_strings.get(i).cloned().unwrap_or_default(),
            rule_comment: codeowners.comment_at(i).map(|s| s.to_string()),
            rule_line_number: codeowners.line_number_at(i),
        },
        None => OwnerResolution::unowned(),
    }
}

/// Convenience: parse and resolve in one call. `codeowners_content` is
/// the raw text (from git or the working tree).
pub fn parse_and_resolve_batch(
    codeowners_content: &str,
    files: &[String],
) -> Vec<OwnerResolution> {
    let codeowners = codeowners_file_parser::from_reader(codeowners_content.as_bytes());
    resolve_owners_batch(&codeowners, files)
}
