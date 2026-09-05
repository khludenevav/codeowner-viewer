//! MCP install for VS Code-family editors (VS Code, Cursor, Windsurf).
//! All use a user-scope `mcp.json` with the same shape.

use std::{fs, path::PathBuf};

use serde_json::{json, Value};

use super::{
    common, path_str, AgentInstallPlan, AgentKind, AgentStatus, InstallState,
    SERVER_KEY,
};

fn variant_dirs(agent: AgentKind) -> Option<Vec<&'static str>> {
    match agent {
        AgentKind::CopilotVscode => Some(vec!["Code"]),
        AgentKind::Cursor => Some(vec!["Cursor"]),
        AgentKind::Windsurf => Some(vec!["Windsurf"]),
        _ => None,
    }
}

fn config_path(agent: AgentKind) -> PathBuf {
    let dirs = variant_dirs(agent).unwrap_or_default();
    let home = super::home_dir().unwrap_or_default();
    let base = if cfg!(target_os = "macos") {
        home.join("Library").join("Application Support")
    } else if cfg!(target_os = "windows") {
        home.join("AppData").join("Roaming")
    } else {
        home.join(".config")
    };
    if let Some(first) = dirs.first() {
        base.join(first).join("User").join("mcp.json")
    } else {
        PathBuf::new()
    }
}

fn read_or_empty(agent: AgentKind) -> Value {
    let text = fs::read_to_string(config_path(agent)).unwrap_or_default();
    if text.trim().is_empty() {
        return json!({});
    }
    serde_json::from_str(&text).unwrap_or_else(|_| json!({}))
}

fn snippet(url: &str) -> String {
    common::pretty_json(&json!({
        "servers": {
            SERVER_KEY: { "type": "http", "url": url }
        }
    }))
}

pub fn status(agent: AgentKind, url: &str) -> AgentStatus {
    let cfg = read_or_empty(agent);
    let entry_url = cfg
        .pointer(&format!("/servers/{SERVER_KEY}/url"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let file = config_path(agent);
    let parent = file.parent().map(|p| p.to_path_buf());
    let parent_exists = parent.as_deref().map(|p| p.exists()).unwrap_or(false);
    let state = if !parent_exists {
        InstallState::Unsupported
    } else {
        match &entry_url {
            Some(u) if u == url => InstallState::Installed,
            Some(_) => InstallState::Conflict,
            None => InstallState::NotInstalled,
        }
    };
    AgentStatus {
        agent,
        display_name: agent.display_name().to_string(),
        state,
        config_path: path_str(&file),
        existing_url: entry_url,
    }
}

pub fn plan(agent: AgentKind, url: &str) -> AgentInstallPlan {
    let file = config_path(agent);
    let existing = status(agent, url).existing_url;
    AgentInstallPlan {
        agent,
        display_name: agent.display_name().to_string(),
        config_path: path_str(&file),
        section: format!("servers.{SERVER_KEY}"),
        language: "json".to_string(),
        snippet: snippet(url),
        existing_url: existing,
        backup_path: None,
        will_backup: file.exists(),
    }
}

pub fn install(agent: AgentKind, url: &str) -> anyhow::Result<AgentInstallPlan> {
    let file = config_path(agent);
    let mut plan = plan(agent, url);
    let backup_path = if file.exists() {
        Some(common::backup(&file)?)
    } else {
        None
    };
    let mut cfg = read_or_empty(agent);
    if !cfg.is_object() {
        cfg = json!({});
    }
    let servers = cfg
        .as_object_mut()
        .unwrap()
        .entry("servers")
        .or_insert_with(|| json!({}));
    if !servers.is_object() {
        *servers = json!({});
    }
    servers.as_object_mut().unwrap().insert(
        SERVER_KEY.to_string(),
        json!({ "type": "http", "url": url }),
    );
    common::atomic_write(&file, &common::pretty_json(&cfg))?;
    plan.backup_path = backup_path.map(|p| path_str(&p));
    Ok(plan)
}

pub fn uninstall(agent: AgentKind) -> anyhow::Result<()> {
    let file = config_path(agent);
    if !file.exists() {
        return Ok(());
    }
    let _ = common::backup(&file)?;
    let mut cfg = read_or_empty(agent);
    if let Some(servers) = cfg
        .as_object_mut()
        .and_then(|o| o.get_mut("servers"))
        .and_then(|v| v.as_object_mut())
    {
        servers.remove(SERVER_KEY);
    }
    common::atomic_write(&file, &common::pretty_json(&cfg))?;
    Ok(())
}
