//! Claude Code MCP install (~/.claude.json).

use std::{fs, path::PathBuf};

use serde_json::{json, Value};

use super::{
    common, path_str, AgentInstallPlan, AgentKind, AgentStatus, InstallState,
    SERVER_KEY,
};

fn config_path() -> PathBuf {
    super::home_dir()
        .map(|h| h.join(".claude.json"))
        .unwrap_or_default()
}

fn read_or_empty() -> Value {
    let path = config_path();
    let text = fs::read_to_string(&path).unwrap_or_default();
    if text.trim().is_empty() {
        return json!({});
    }
    serde_json::from_str(&text).unwrap_or_else(|_| json!({}))
}

fn snippet(url: &str) -> String {
    let value = json!({
        "mcpServers": {
            SERVER_KEY: { "type": "http", "url": url }
        }
    });
    common::pretty_json(&value)
}

pub fn status(url: &str) -> AgentStatus {
    let cfg = read_or_empty();
    let entry_url = cfg
        .pointer(&format!("/mcpServers/{SERVER_KEY}/url"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let state = match &entry_url {
        Some(u) if u == url => InstallState::Installed,
        Some(_) => InstallState::Conflict,
        None => InstallState::NotInstalled,
    };
    AgentStatus {
        agent: AgentKind::ClaudeCode,
        display_name: AgentKind::ClaudeCode.display_name().to_string(),
        state,
        config_path: path_str(&config_path()),
        existing_url: entry_url,
    }
}

pub fn plan(url: &str) -> AgentInstallPlan {
    let file = config_path();
    let existing_url = status(url).existing_url;
    AgentInstallPlan {
        agent: AgentKind::ClaudeCode,
        display_name: AgentKind::ClaudeCode.display_name().to_string(),
        config_path: path_str(&file),
        section: format!("mcpServers.{SERVER_KEY}"),
        language: "json".to_string(),
        snippet: snippet(url),
        existing_url,
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
    let mut cfg = read_or_empty();
    if !cfg.is_object() {
        cfg = json!({});
    }
    let mcp_servers = cfg
        .as_object_mut()
        .unwrap()
        .entry("mcpServers")
        .or_insert_with(|| json!({}));
    if !mcp_servers.is_object() {
        *mcp_servers = json!({});
    }
    mcp_servers
        .as_object_mut()
        .unwrap()
        .insert(
            SERVER_KEY.to_string(),
            json!({ "type": "http", "url": url }),
        );
    common::atomic_write(&file, &common::pretty_json(&cfg))?;
    plan.backup_path = backup_path.map(|p| path_str(&p));
    Ok(plan)
}

pub fn uninstall() -> anyhow::Result<()> {
    let file = config_path();
    if !file.exists() {
        return Ok(());
    }
    let _ = common::backup(&file)?;
    let mut cfg = read_or_empty();
    if let Some(mcp_servers) = cfg
        .as_object_mut()
        .and_then(|o| o.get_mut("mcpServers"))
        .and_then(|v| v.as_object_mut())
    {
        mcp_servers.remove(SERVER_KEY);
    }
    common::atomic_write(&file, &common::pretty_json(&cfg))?;
    Ok(())
}
