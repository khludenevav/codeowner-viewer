//! Configuration file writers for supported AI coding agents.
//!
//! Every targeted agent (Claude Code, Codex CLI, GitHub Copilot in
//! VS Code, Cursor, Windsurf) speaks Streamable HTTP MCP, so every
//! install is just an `{ type: "http", url }` entry — no stdio proxy
//! required.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

pub mod claude;
pub mod codex;
pub mod common;
pub mod vscode_family;

pub const SERVER_KEY: &str = "codeowners-viewer";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    ClaudeCode,
    CodexCli,
    CopilotVscode,
    Cursor,
    Windsurf,
}

impl AgentKind {
    pub fn display_name(self) -> &'static str {
        match self {
            AgentKind::ClaudeCode => "Claude Code",
            AgentKind::CodexCli => "Codex CLI",
            AgentKind::CopilotVscode => "GitHub Copilot in VS Code",
            AgentKind::Cursor => "Cursor",
            AgentKind::Windsurf => "Windsurf",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum InstallState {
    /// The agent's config already points at our current URL.
    Installed,
    /// No entry present.
    NotInstalled,
    /// Entry present but pointing at a different URL.
    Conflict,
    /// Agent isn't available on this machine (config dir missing).
    Unsupported,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct AgentStatus {
    pub agent: AgentKind,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub state: InstallState,
    /// Absolute path to the agent's config file we would edit.
    #[serde(rename = "configPath")]
    pub config_path: String,
    /// If an entry already exists, its current URL (may equal ours →
    /// `Installed`, or differ → `Conflict`).
    #[serde(rename = "existingUrl", skip_serializing_if = "Option::is_none")]
    pub existing_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct AgentInstallPlan {
    pub agent: AgentKind,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "configPath")]
    pub config_path: String,
    /// Section / TOML table / JSON key that will be created or updated.
    pub section: String,
    /// Config-file syntax (`json` or `toml`) — the frontend uses this
    /// to render the snippet with the right highlight.
    pub language: String,
    /// The exact snippet that will be written into the file (only the
    /// added/updated block, not the whole file).
    pub snippet: String,
    /// If an existing entry conflicts, its previous URL.
    #[serde(rename = "existingUrl", skip_serializing_if = "Option::is_none")]
    pub existing_url: Option<String>,
    #[serde(rename = "backupPath", skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
    #[serde(rename = "willBackup")]
    pub will_backup: bool,
}

pub fn status(agent: AgentKind, url: &str) -> AgentStatus {
    match agent {
        AgentKind::ClaudeCode => claude::status(url),
        AgentKind::CodexCli => codex::status(url),
        AgentKind::CopilotVscode | AgentKind::Cursor | AgentKind::Windsurf => {
            vscode_family::status(agent, url)
        }
    }
}

pub fn plan_install(agent: AgentKind, url: &str) -> AgentInstallPlan {
    match agent {
        AgentKind::ClaudeCode => claude::plan(url),
        AgentKind::CodexCli => codex::plan(url),
        AgentKind::CopilotVscode | AgentKind::Cursor | AgentKind::Windsurf => {
            vscode_family::plan(agent, url)
        }
    }
}

pub fn install(agent: AgentKind, url: &str) -> anyhow::Result<AgentInstallPlan> {
    match agent {
        AgentKind::ClaudeCode => claude::install(url),
        AgentKind::CodexCli => codex::install(url),
        AgentKind::CopilotVscode | AgentKind::Cursor | AgentKind::Windsurf => {
            vscode_family::install(agent, url)
        }
    }
}

pub fn uninstall(agent: AgentKind) -> anyhow::Result<()> {
    match agent {
        AgentKind::ClaudeCode => claude::uninstall(),
        AgentKind::CodexCli => codex::uninstall(),
        AgentKind::CopilotVscode | AgentKind::Cursor | AgentKind::Windsurf => {
            vscode_family::uninstall(agent)
        }
    }
}

/// User home directory resolver used by every installer. Kept in the
/// top-level module so tests can override it via env var if needed.
pub(crate) fn home_dir() -> Option<PathBuf> {
    #[allow(deprecated)]
    std::env::home_dir()
}

/// Absolute string path, or empty string if we cannot resolve it — the
/// UI treats an empty string as "unsupported".
pub(crate) fn path_str(p: &Path) -> String {
    p.to_str().unwrap_or("").to_string()
}
