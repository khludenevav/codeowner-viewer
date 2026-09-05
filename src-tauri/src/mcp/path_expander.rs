//! Expand a mixed list of `paths` (files, directory prefixes, globs) into
//! a concrete, deduped, sorted list of repo-root-relative file paths.

use std::collections::BTreeSet;

use glob::Pattern;

use super::error::McpToolError;

/// Soft cap on the number of expanded files. Anything above this returns
/// [`McpToolError::TooManyPaths`] so a runaway glob can't wedge the
/// server.
pub const MAX_EXPANDED_PATHS: usize = 500_000;

/// Expand `paths` against `reachable_files` (all repo-relative). Also
/// enforces path-safety rules (no absolute, no `..`, no backslashes on
/// non-Windows). Pass an empty `paths` slice to get *all* reachable
/// files (used by `for = "branch"` when the caller doesn't restrict the
/// scope).
///
/// `include_all_when_paths_empty` controls the meaning of an empty
/// `paths` slice: `true` for `for = "branch"`, `false` for
/// `for = "changed_files"` (which uses the changed set as its own base
/// and treats `paths` as additive).
pub fn expand_paths(
    paths: &[String],
    reachable_files: &[String],
    include_all_when_paths_empty: bool,
) -> Result<Vec<String>, McpToolError> {
    if paths.is_empty() {
        if include_all_when_paths_empty {
            return enforce_cap(reachable_files.to_vec());
        }
        return Ok(Vec::new());
    }

    validate_paths(paths)?;

    // Build a set for fast presence checks, and a sorted directory index
    // for prefix lookups.
    let file_set: BTreeSet<&str> =
        reachable_files.iter().map(|s| s.as_str()).collect();

    let mut out: BTreeSet<String> = BTreeSet::new();
    for entry in paths {
        let cleaned = entry.trim_start_matches("./").trim_end_matches('/');
        if is_glob(cleaned) {
            // Compile once, filter reachable set.
            let pattern = Pattern::new(cleaned).map_err(|e| {
                McpToolError::InvalidArgs(format!(
                    "invalid glob `{entry}`: {e}"
                ))
            })?;
            for f in reachable_files {
                if pattern.matches(f) {
                    out.insert(f.clone());
                }
            }
        } else if is_directory_prefix(cleaned, &file_set) {
            let prefix = format!("{cleaned}/");
            for f in reachable_files {
                if f.starts_with(&prefix) {
                    out.insert(f.clone());
                }
            }
        } else if file_set.contains(cleaned) {
            out.insert(cleaned.to_string());
        }
        // else: silently drop entries that resolve to nothing — the
        // response will just not contain them.
    }

    enforce_cap(out.into_iter().collect())
}

fn is_glob(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

fn is_directory_prefix(cleaned: &str, files: &BTreeSet<&str>) -> bool {
    if cleaned.is_empty() {
        return true;
    }
    let prefix = format!("{cleaned}/");
    // BTreeSet::range would be ideal here; for simplicity we do a scan
    // which is bounded by O(files) — the caller already limits us to
    // sane sizes via the 500k cap.
    files.iter().any(|f| f.starts_with(&prefix))
}

fn validate_paths(paths: &[String]) -> Result<(), McpToolError> {
    for p in paths {
        if p.starts_with('/') {
            return Err(McpToolError::PathOutsideRepo(p.clone()));
        }
        #[cfg(not(windows))]
        if p.contains('\\') {
            return Err(McpToolError::PathOutsideRepo(p.clone()));
        }
        for seg in p.split('/') {
            if seg == ".." {
                return Err(McpToolError::PathOutsideRepo(p.clone()));
            }
        }
    }
    Ok(())
}

fn enforce_cap(mut v: Vec<String>) -> Result<Vec<String>, McpToolError> {
    if v.len() > MAX_EXPANDED_PATHS {
        return Err(McpToolError::TooManyPaths(v.len(), MAX_EXPANDED_PATHS));
    }
    v.sort();
    v.dedup();
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> Vec<String> {
        vec![
            "a/b/c.txt".into(),
            "a/b/d.txt".into(),
            "a/e.txt".into(),
            "z/w.txt".into(),
        ]
    }

    #[test]
    fn empty_paths_with_include_all_returns_reachable() {
        let out = expand_paths(&[], &files(), true).unwrap();
        assert_eq!(out.len(), 4);
    }

    #[test]
    fn empty_paths_without_include_all_returns_empty() {
        let out = expand_paths(&[], &files(), false).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn expands_directory_recursively() {
        let out = expand_paths(&["a/b".into()], &files(), true).unwrap();
        assert_eq!(out, vec!["a/b/c.txt".to_string(), "a/b/d.txt".into()]);
    }

    #[test]
    fn expands_glob() {
        let out = expand_paths(&["**/*.txt".into()], &files(), true).unwrap();
        assert_eq!(out.len(), 4);
    }

    #[test]
    fn resolves_exact_file() {
        let out = expand_paths(&["a/e.txt".into()], &files(), true).unwrap();
        assert_eq!(out, vec!["a/e.txt".to_string()]);
    }

    #[test]
    fn rejects_absolute() {
        let err = expand_paths(&["/etc/passwd".into()], &files(), true).unwrap_err();
        matches!(err, McpToolError::PathOutsideRepo(_));
    }

    #[test]
    fn rejects_dot_dot() {
        let err = expand_paths(&["../foo".into()], &files(), true).unwrap_err();
        matches!(err, McpToolError::PathOutsideRepo(_));
    }
}
