//! Serde types describing the MCP `get_codeowners` tool input/output.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ForScope {
    /// Resolve ownership at the given git ref (defaults to `HEAD`).
    Branch,
    /// Resolve ownership for staged + unstaged + untracked files.
    ChangedFiles,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ResponseMode {
    /// Grouped by owner-set, directory-collapsed when possible.
    Compact,
    /// One entry per file with owners + rule comment.
    Normal,
    /// Normal, plus which CODEOWNERS line matched.
    Full,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct GetCodeownersInput {
    /// Absolute path to a repo checkout. Mandatory.
    pub repo: String,

    /// Whether to scope the request to a git ref (`branch`) or to the
    /// working-tree changed set (`changed_files`).
    #[serde(rename = "for")]
    pub for_scope: ForScope,

    /// Optional. Defaults to `HEAD`.
    ///
    /// When `for = "branch"`, this is the ref to enumerate files from.
    /// When `for = "changed_files"`, this is the ref that additional
    /// `paths` entries are resolved against.
    #[serde(default)]
    pub branch: Option<String>,

    /// Repo-root-relative paths. Each entry may be a file, a directory
    /// (recursive), or a glob. Absolute paths and `..` are rejected.
    #[serde(default)]
    pub paths: Option<Vec<String>>,

    /// How verbose the response should be.
    #[serde(rename = "responseMode")]
    pub response_mode: ResponseMode,
}

/// Compact-mode entry: grouped by (owners, ruleComment), listing files
/// or directories that share exactly that ownership.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CompactEntry {
    /// Space-separated list of owners. Empty string means unowned.
    pub owners: String,
    /// Repo-root-relative file or directory paths (never globs).
    pub paths: Vec<String>,
    #[serde(rename = "ruleComment", skip_serializing_if = "Option::is_none")]
    pub rule_comment: Option<String>,
}

/// Normal-mode per-file entry.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NormalEntry {
    pub owners: String,
    #[serde(rename = "ruleComment", skip_serializing_if = "Option::is_none")]
    pub rule_comment: Option<String>,
}

/// Full-mode per-file entry (normal + line number).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FullEntry {
    pub owners: String,
    #[serde(rename = "ruleComment", skip_serializing_if = "Option::is_none")]
    pub rule_comment: Option<String>,
    #[serde(rename = "ruleLineNumber")]
    pub rule_line_number: i64,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum GetCodeownersOutput {
    Compact(Vec<CompactEntry>),
    Normal(BTreeMap<String, NormalEntry>),
    Full(BTreeMap<String, FullEntry>),
}
