//! Serde types describing the MCP `get_codeowners` tool input.
//!
//! The tool no longer emits structured JSON: its output is a plain DSL
//! text body produced by the `dsl_emitter` module. See
//! `dsl_emitter.rs` for the grammar.

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

/// Verbosity of the DSL body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ResponseMode {
    /// Only files/dirs whose rule differs from the enclosing default
    /// are emitted. Directories may declare their own `_default`.
    #[default]
    Compact,
    /// Every requested file is emitted with its rule id.
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

    /// Verbosity of the DSL body. Defaults to `compact`.
    #[serde(rename = "responseMode", default)]
    pub response_mode: ResponseMode,

    /// Optional. When set, any subtree that is deeper than `maxDepth`
    /// levels below a caller-provided path AND that mixes multiple
    /// rules is replaced by an `[id:count,...] TRUNCATED` marker.
    /// Depth 0 = the requested path itself; depth 1 = its direct
    /// children. Applied per each entry of `paths`. If `paths` is
    /// empty or missing, `maxDepth` is ignored.
    #[serde(rename = "maxDepth", default)]
    pub max_depth: Option<u32>,
}

/// Input for the `export_codeowners` tool. Always full-repo dump; no
/// `paths`, no `maxDepth`, no `responseMode`. Optional owner /
/// extension filters shrink the `files` map (and, transitively, the
/// `rules` table).
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
pub struct ExportCodeownersInput {
    /// Absolute path to a repo checkout. Mandatory.
    pub repo: String,

    /// Optional owner filter. A file is included when ANY of its
    /// owners matches ANY entry here. Handles are compared as-is
    /// (case-sensitive, `@`-prefixed).
    #[serde(default)]
    pub owners: Option<Vec<String>>,

    /// Optional extension filter. Case-insensitive; a leading dot is
    /// ignored (`.ts` == `TS` == `ts`).
    #[serde(default)]
    pub extensions: Option<Vec<String>>,

    /// Optional absolute path where the dump file should be written.
    /// When set, the file is created (or overwritten if it already
    /// exists) at that exact location. Parent directories are created
    /// as needed. When omitted, a fresh file with a generated name is
    /// written under the OS temp dir and stale dumps are pruned.
    #[serde(default)]
    pub path: Option<String>,
}

/// Output for the `export_codeowners` tool. Response body is a small
/// JSON blob pointing the agent at the dumped file — the actual data
/// lives on disk.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExportCodeownersOutput {
    /// Absolute path to the JSON file that was just written.
    pub path: String,
    /// Byte length of the JSON file on disk.
    #[serde(rename = "sizeBytes")]
    pub size_bytes: u64,
    /// Number of files recorded in the export (post-filter).
    #[serde(rename = "fileCount")]
    pub file_count: usize,
    /// Number of rules recorded in the export (post-filter). Includes
    /// the reserved rule `0` when any unowned files are present.
    #[serde(rename = "ruleCount")]
    pub rule_count: usize,
    /// Schema tag matching the `schema` field inside the JSON file
    /// itself.
    pub schema: String,
}

/// Input for the `list_owners` tool. Bare-minimum discovery tool that
/// returns the distinct owner handles present in the repo at HEAD.
/// No filters — the whole point is to enumerate the space of valid
/// owner strings before calling `export_codeowners` with `owners[]`.
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
pub struct ListOwnersInput {
    /// Absolute path to a repo checkout. Mandatory.
    pub repo: String,
}

/// Output for the `list_owners` tool. Intentionally minimal.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListOwnersOutput {
    /// Distinct owner handles, alphabetically sorted. Unowned files
    /// do NOT contribute an entry.
    pub owners: Vec<String>,
}

/// Input for the `owners_stats` tool.
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
pub struct OwnersStatsInput {
    /// Absolute path to a repo checkout. Mandatory.
    pub repo: String,
}

/// Per-owner file counts for the `owners_stats` map. Wrapped in an
/// object (rather than storing the count directly on the map value) so
/// the shape is extensible.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OwnerStat {
    /// Number of files this owner appears on (as a full or co-owner).
    pub files: usize,
}

/// Output for the `owners_stats` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OwnersStatsOutput {
    /// Total files in the repo at HEAD.
    #[serde(rename = "totalFiles")]
    pub total_files: usize,
    /// Files without any matching CODEOWNERS rule.
    #[serde(rename = "unownedFiles")]
    pub unowned_files: usize,
    /// Owner handle → stats. Emitted as a JSON object keyed by owner
    /// handle (alphabetically sorted by the `BTreeMap`). Sum of
    /// `files` may exceed `totalFiles` because a file with N
    /// co-owners contributes +1 to each of those N owners.
    pub owners: std::collections::BTreeMap<String, OwnerStat>,
}
