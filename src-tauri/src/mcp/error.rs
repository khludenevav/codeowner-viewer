//! Structured errors for the MCP `get_codeowners` tool. Serialized into
//! the tool-level error content so an agent gets a specific, actionable
//! reason instead of a generic RPC error.

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Clone)]
#[serde(tag = "code", content = "message")]
pub enum McpToolError {
    #[error("`{0}` is not a git repository")]
    RepoNotAGitRepo(String),
    #[error("branch `{0}` was not found in this repo")]
    BranchNotFound(String),
    #[error("CODEOWNERS file `{0}` was not found in this repo")]
    CodeownersMissing(String),
    #[error("path `{0}` escapes the repo root or is absolute")]
    PathOutsideRepo(String),
    #[error("too many paths after expansion ({0}); soft cap is {1}")]
    TooManyPaths(usize, usize),
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("internal error: {0}")]
    Internal(String),
}
