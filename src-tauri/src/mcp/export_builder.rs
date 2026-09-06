//! Shared JSON export builder for the `export_codeowners` MCP tool AND
//! the UI "Export to json..." button. Both go through this module so
//! their output is byte-identical.
//!
//! Produces the `codeowners-export/v1` schema documented in
//! [`SERVER_INSTRUCTIONS`](super::SERVER_INSTRUCTIONS). See the module
//! test suite for shape examples.
//!
//! Rule keys are the 1-based CODEOWNERS line numbers, matching the
//! vocabulary already used by the DSL `get_codeowners` tool. Rule `0`
//! is a reserved sentinel meaning "no matching CODEOWNERS rule" and
//! unowned files map to it.

use std::collections::BTreeMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::codeowners_engine::{
    self, OwnerResolution,
};
use crate::codeowners_file_parser;

use super::{error::McpToolError, repo_resolver::ResolvedRepo, tool_get_codeowners::HEAD_REF};

pub const SCHEMA_VERSION: &str = "codeowners-export/v1";
pub const UNOWNED_RULE_ID: u32 = 0;

/// Top-level export payload. Serialized as JSON to the dump file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPayload {
    pub schema: String,
    /// Rule table keyed by CODEOWNERS 1-based line number (as string
    /// since JSON object keys must be strings). Key `"0"` is the
    /// reserved unowned sentinel and is only present when at least one
    /// unowned file remains after filtering.
    pub rules: BTreeMap<String, RulePayload>,
    /// Repo-root-relative file paths mapped to their rule id.
    /// `BTreeMap` gives deterministic (alphabetical) output.
    pub files: BTreeMap<String, u32>,
    pub stats: StatsPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePayload {
    /// Individual owner handles. Empty vec for the unowned sentinel.
    pub owners: Vec<String>,
    /// Verbatim inline `# ...` comment from the CODEOWNERS rule (with
    /// the leading `#` and whitespace stripped). May contain
    /// project-specific markers such as `!required` or extra
    /// `@owner` mentions used as notes. `null` when the rule had no
    /// trailing comment.
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsPayload {
    pub total_files: usize,
    pub owned_files: usize,
    pub unowned_files: usize,
    pub rule_count: usize,
    pub codeowners_file_path: String,
    /// True when input owners/extensions filter was non-empty.
    pub filtered: bool,
    /// ISO-8601 UTC timestamp of when this dump was produced,
    /// e.g. `2026-09-06T18:15:28Z`.
    pub generated_at: String,
}

/// Filters applied to which files land in the output. Both are optional.
/// Semantics: OR within each list, AND between the two.
#[derive(Debug, Clone, Default)]
pub struct ExportFilters {
    /// Match if any of the file's owners is in this set. Empty/None
    /// means no owner filter.
    pub owners: Option<Vec<String>>,
    /// Match if the file's extension is in this set. Compared
    /// case-insensitively, leading dot ignored (`"java"` matches
    /// `"foo.Java"`). Empty/None means no extension filter.
    pub extensions: Option<Vec<String>>,
}

impl ExportFilters {
    pub fn any_active(&self) -> bool {
        self.owners.as_ref().is_some_and(|v| !v.is_empty())
            || self.extensions.as_ref().is_some_and(|v| !v.is_empty())
    }
}

/// Build the export payload for a resolved repo + branch.
///
/// `branch` may be an empty string; defaults to `HEAD`.
pub fn build_export_payload(
    repo: &ResolvedRepo,
    branch: &str,
    filters: &ExportFilters,
) -> Result<ExportPayload, McpToolError> {
    let repo_path_str = repo
        .abs_repo_path
        .to_str()
        .ok_or_else(|| McpToolError::Internal("repo path is not UTF-8".into()))?;

    let effective_branch = if branch.is_empty() { HEAD_REF } else { branch };

    let files = codeowners_engine::get_branch_files_vector(repo_path_str, effective_branch);
    if files.is_empty() && !rev_exists(repo_path_str, effective_branch) {
        return Err(McpToolError::BranchNotFound(effective_branch.to_string()));
    }

    let content = codeowners_engine::get_codeowners_content_at_ref(
        repo_path_str,
        effective_branch,
        &repo.codeowners_rel,
    );
    if content.is_empty() {
        return Err(McpToolError::CodeownersMissing(repo.codeowners_rel.clone()));
    }

    let codeowners = codeowners_file_parser::from_reader(content.as_bytes());
    let resolved = codeowners_engine::resolve_owners_batch(&codeowners, &files);

    Ok(assemble_payload(&files, &resolved, &repo.codeowners_rel, filters))
}

fn assemble_payload(
    files: &[String],
    resolved: &[OwnerResolution],
    codeowners_rel: &str,
    filters: &ExportFilters,
) -> ExportPayload {
    debug_assert_eq!(files.len(), resolved.len());

    let normalized_owners = filters
        .owners
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(|v| {
            v.iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        });
    let normalized_exts = filters
        .extensions
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(|v| {
            v.iter()
                .map(|s| normalize_ext(s))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        });

    let mut files_map: BTreeMap<String, u32> = BTreeMap::new();
    let mut rules: BTreeMap<String, RulePayload> = BTreeMap::new();
    let mut owned_count = 0usize;
    let mut unowned_count = 0usize;
    let mut unowned_seen = false;

    for (path, res) in files.iter().zip(resolved.iter()) {
        let owners_vec = split_owners(&res.owners);

        if let Some(filter_owners) = normalized_owners.as_ref() {
            if !owners_vec.iter().any(|o| filter_owners.iter().any(|f| f == o)) {
                continue;
            }
        }

        if let Some(filter_exts) = normalized_exts.as_ref() {
            let ext = extract_ext(path);
            if !filter_exts.iter().any(|f| f == &ext) {
                continue;
            }
        }

        let rule_id = if res.rule_line_number == 0 {
            UNOWNED_RULE_ID
        } else {
            res.rule_line_number
        };

        files_map.insert(path.clone(), rule_id);

        if rule_id == UNOWNED_RULE_ID {
            unowned_count += 1;
            unowned_seen = true;
        } else {
            owned_count += 1;
        }

        rules.entry(rule_id.to_string()).or_insert_with(|| RulePayload {
            owners: owners_vec,
            comment: res.rule_comment.clone(),
        });
    }

    if unowned_seen {
        rules
            .entry(UNOWNED_RULE_ID.to_string())
            .or_insert_with(|| RulePayload { owners: Vec::new(), comment: None });
    }

    let total = files_map.len();
    let rule_count = rules.len();

    let stats = StatsPayload {
        total_files: total,
        owned_files: owned_count,
        unowned_files: unowned_count,
        rule_count,
        codeowners_file_path: codeowners_rel.to_string(),
        filtered: filters.any_active(),
        generated_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };

    ExportPayload {
        schema: SCHEMA_VERSION.to_string(),
        rules,
        files: files_map,
        stats,
    }
}

/// Split the `", "`-separated owners string produced by
/// `Owners::owner_strings`. Returns an empty vec when the input is
/// empty/whitespace.
fn split_owners(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    trimmed
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Lowercase, no leading dot. Returns empty string when the file has
/// no extension.
fn normalize_ext(raw: &str) -> String {
    let trimmed = raw.trim();
    let no_dot = trimmed.strip_prefix('.').unwrap_or(trimmed);
    no_dot.to_ascii_lowercase()
}

/// Extract the extension of `path` (the segment after the LAST `.` in
/// the basename), lowercased, no leading dot. Returns empty string
/// when there is no `.` in the basename or the file starts with a dot.
fn extract_ext(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    let dot = match file.rfind('.') {
        Some(i) if i > 0 => i,
        _ => return String::new(),
    };
    file[dot + 1..].to_ascii_lowercase()
}

fn rev_exists(abs_repo_path: &str, rev: &str) -> bool {
    let out = std::process::Command::new("git")
        .current_dir(abs_repo_path)
        .arg("rev-parse")
        .arg("--verify")
        .arg(rev)
        .output();
    match out {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res(owners: &str, comment: Option<&str>, line: u32) -> OwnerResolution {
        OwnerResolution {
            owners: owners.to_string(),
            rule_comment: comment.map(|s| s.to_string()),
            rule_line_number: line,
        }
    }

    fn unowned() -> OwnerResolution {
        OwnerResolution::unowned()
    }

    #[test]
    fn full_repo_shape() {
        let files = vec![
            "app/src/Main.java".to_string(),
            "app/src/util/Helper.java".to_string(),
            "app/orphan.txt".to_string(),
        ];
        let resolved = vec![
            res("@fivetran/kepler", Some("!required"), 42),
            res("@fivetran/kepler", Some("!required"), 42),
            unowned(),
        ];
        let filters = ExportFilters::default();

        let payload = assemble_payload(&files, &resolved, ".github/CODEOWNERS", &filters);

        assert_eq!(payload.schema, "codeowners-export/v1");
        assert_eq!(payload.stats.total_files, 3);
        assert_eq!(payload.stats.owned_files, 2);
        assert_eq!(payload.stats.unowned_files, 1);
        assert_eq!(payload.stats.rule_count, 2);
        assert!(!payload.stats.filtered);
        assert_eq!(payload.stats.codeowners_file_path, ".github/CODEOWNERS");
        assert!(payload.stats.generated_at.ends_with('Z'));
        assert_eq!(payload.stats.generated_at.len(), 20); // YYYY-MM-DDTHH:MM:SSZ

        assert_eq!(payload.files["app/src/Main.java"], 42);
        assert_eq!(payload.files["app/src/util/Helper.java"], 42);
        assert_eq!(payload.files["app/orphan.txt"], 0);

        let rule42 = &payload.rules["42"];
        assert_eq!(rule42.owners, vec!["@fivetran/kepler"]);
        assert_eq!(rule42.comment.as_deref(), Some("!required"));

        let rule0 = &payload.rules["0"];
        assert!(rule0.owners.is_empty());
        assert!(rule0.comment.is_none());
    }

    #[test]
    fn multi_owner_split() {
        let files = vec!["app/x.java".to_string()];
        let resolved = vec![res("@fivetran/a, @fivetran/b, @fivetran/c", None, 5)];
        let payload = assemble_payload(&files, &resolved, "CODEOWNERS", &ExportFilters::default());
        assert_eq!(
            payload.rules["5"].owners,
            vec!["@fivetran/a", "@fivetran/b", "@fivetran/c"]
        );
    }

    #[test]
    fn filter_by_owner() {
        let files = vec!["a.java".to_string(), "b.ts".to_string(), "c.py".to_string()];
        let resolved = vec![
            res("@team/alpha", None, 10),
            res("@team/beta", None, 20),
            res("@team/alpha, @team/beta", None, 30),
        ];
        let filters = ExportFilters {
            owners: Some(vec!["@team/alpha".to_string()]),
            extensions: None,
        };

        let payload = assemble_payload(&files, &resolved, "CODEOWNERS", &filters);
        assert!(payload.stats.filtered);
        assert_eq!(payload.files.len(), 2);
        assert!(payload.files.contains_key("a.java"));
        assert!(payload.files.contains_key("c.py"));
        assert!(!payload.files.contains_key("b.ts"));
        // Rules table only holds rules 10 and 30.
        assert_eq!(payload.rules.len(), 2);
        assert!(payload.rules.contains_key("10"));
        assert!(payload.rules.contains_key("30"));
        assert!(!payload.rules.contains_key("20"));
    }

    #[test]
    fn filter_by_extension() {
        let files = vec![
            "a.java".to_string(),
            "b.TS".to_string(),
            "c.ts".to_string(),
            "d.py".to_string(),
            "no_ext_file".to_string(),
        ];
        let resolved = vec![
            res("@t/a", None, 1),
            res("@t/b", None, 2),
            res("@t/c", None, 3),
            res("@t/d", None, 4),
            res("@t/e", None, 5),
        ];
        let filters = ExportFilters {
            owners: None,
            extensions: Some(vec![".ts".to_string(), "PY".to_string()]),
        };

        let payload = assemble_payload(&files, &resolved, "CODEOWNERS", &filters);
        assert!(payload.stats.filtered);
        assert_eq!(payload.files.len(), 3);
        assert!(payload.files.contains_key("b.TS"));
        assert!(payload.files.contains_key("c.ts"));
        assert!(payload.files.contains_key("d.py"));
        assert!(!payload.files.contains_key("a.java"));
        assert!(!payload.files.contains_key("no_ext_file"));
    }

    #[test]
    fn filter_by_both_and() {
        let files = vec![
            "a.ts".to_string(),
            "b.ts".to_string(),
            "c.java".to_string(),
            "d.java".to_string(),
        ];
        let resolved = vec![
            res("@t/alpha", None, 1),
            res("@t/beta", None, 2),
            res("@t/alpha", None, 3),
            res("@t/beta", None, 4),
        ];
        let filters = ExportFilters {
            owners: Some(vec!["@t/alpha".to_string()]),
            extensions: Some(vec!["ts".to_string()]),
        };

        let payload = assemble_payload(&files, &resolved, "CODEOWNERS", &filters);
        // Only a.ts satisfies BOTH filters.
        assert_eq!(payload.files.len(), 1);
        assert!(payload.files.contains_key("a.ts"));
    }

    #[test]
    fn unowned_only_when_present_after_filter() {
        let files = vec!["a.java".to_string(), "orphan.txt".to_string()];
        let resolved = vec![res("@t/a", None, 1), unowned()];
        // Filter that keeps only owned files → rule 0 must NOT appear.
        let filters = ExportFilters {
            owners: Some(vec!["@t/a".to_string()]),
            extensions: None,
        };
        let payload = assemble_payload(&files, &resolved, "CODEOWNERS", &filters);
        assert!(!payload.rules.contains_key("0"));
        assert_eq!(payload.stats.unowned_files, 0);
    }

    #[test]
    fn json_roundtrip() {
        let files = vec!["a.java".to_string()];
        let resolved = vec![res("@t/a", Some("(note)"), 7)];
        let payload = assemble_payload(&files, &resolved, "CODEOWNERS", &ExportFilters::default());
        let json = serde_json::to_string(&payload).unwrap();
        let back: ExportPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.schema, "codeowners-export/v1");
        assert_eq!(back.files["a.java"], 7);
        assert_eq!(back.rules["7"].owners, vec!["@t/a"]);
    }

    #[test]
    fn extract_ext_edge_cases() {
        assert_eq!(extract_ext("foo/bar/baz.rs"), "rs");
        assert_eq!(extract_ext("foo/bar/baz"), "");
        assert_eq!(extract_ext(".gitignore"), "");
        assert_eq!(extract_ext("baz.tar.gz"), "gz");
        assert_eq!(extract_ext("dir/foo.RS"), "rs");
    }

    #[test]
    fn split_owners_edge_cases() {
        assert!(split_owners("").is_empty());
        assert!(split_owners("   ").is_empty());
        assert_eq!(split_owners("@a"), vec!["@a"]);
        assert_eq!(split_owners("@a, @b"), vec!["@a", "@b"]);
        assert_eq!(split_owners("@a,@b"), vec!["@a", "@b"]);
        assert_eq!(split_owners("  @a ,, @b  "), vec!["@a", "@b"]);
    }
}
