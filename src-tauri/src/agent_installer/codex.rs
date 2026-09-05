//! Codex CLI MCP install (~/.codex/config.toml).

use std::{fs, path::PathBuf};

use toml_edit::{DocumentMut, Item, Table};

use super::{
    common, path_str, AgentInstallPlan, AgentKind, AgentStatus, InstallState,
    SERVER_KEY,
};

fn config_path() -> PathBuf {
    super::home_dir()
        .map(|h| h.join(".codex").join("config.toml"))
        .unwrap_or_default()
}

fn read_or_empty() -> DocumentMut {
    let path = config_path();
    let text = fs::read_to_string(&path).unwrap_or_default();
    text.parse::<DocumentMut>().unwrap_or_default()
}

fn snippet(url: &str) -> String {
    format!(
        "[mcp_servers.{SERVER_KEY}]\nurl = \"{url}\"\n"
    )
}

fn existing_url_from(doc: &DocumentMut) -> Option<String> {
    doc.get("mcp_servers")
        .and_then(|i| i.as_table())
        .and_then(|t| t.get(SERVER_KEY))
        .and_then(|i| i.as_table_like())
        .and_then(|t| t.get("url"))
        .and_then(|i| i.as_str())
        .map(|s| s.to_string())
}

pub fn status(url: &str) -> AgentStatus {
    let doc = read_or_empty();
    let existing_url = existing_url_from(&doc);
    let state = match &existing_url {
        Some(u) if u == url => InstallState::Installed,
        Some(_) => InstallState::Conflict,
        None => InstallState::NotInstalled,
    };
    AgentStatus {
        agent: AgentKind::CodexCli,
        display_name: AgentKind::CodexCli.display_name().to_string(),
        state,
        config_path: path_str(&config_path()),
        existing_url,
    }
}

pub fn plan(url: &str) -> AgentInstallPlan {
    let file = config_path();
    let existing = existing_url_from(&read_or_empty());
    AgentInstallPlan {
        agent: AgentKind::CodexCli,
        display_name: AgentKind::CodexCli.display_name().to_string(),
        config_path: path_str(&file),
        section: format!("[mcp_servers.{SERVER_KEY}]"),
        language: "toml".to_string(),
        snippet: snippet(url),
        existing_url: existing,
        backup_path: None,
        will_backup: file.exists(),
    }
}

pub fn install(url: &str) -> anyhow::Result<AgentInstallPlan> {
    let file = config_path();
    let mut plan = plan(url);
    let backup_path = if file.exists() {
        Some(common::backup(&file)?)
    } else {
        None
    };
    let mut doc = read_or_empty();

    // Ensure top-level [mcp_servers] exists as a regular table.
    if doc.get("mcp_servers").is_none() {
        let mut t = Table::new();
        t.set_implicit(true);
        doc.insert("mcp_servers", Item::Table(t));
    }
    let mcp_servers = doc
        .get_mut("mcp_servers")
        .and_then(|i| i.as_table_mut())
        .ok_or_else(|| anyhow::anyhow!("`mcp_servers` is not a TOML table"))?;

    // Overwrite (or create) the sub-table.
    let mut entry = Table::new();
    entry["url"] = toml_edit::value(url);
    mcp_servers.insert(SERVER_KEY, Item::Table(entry));

    common::atomic_write(&file, &doc.to_string())?;
    plan.backup_path = backup_path.map(|p| path_str(&p));
    Ok(plan)
}

pub fn uninstall() -> anyhow::Result<()> {
    let file = config_path();
    if !file.exists() {
        return Ok(());
    }
    let _ = common::backup(&file)?;
    let mut doc = read_or_empty();
    if let Some(mcp_servers) = doc
        .get_mut("mcp_servers")
        .and_then(|i| i.as_table_mut())
    {
        mcp_servers.remove(SERVER_KEY);
    }
    common::atomic_write(&file, &doc.to_string())?;
    Ok(())
}
