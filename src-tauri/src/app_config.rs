//! Rust-side view of the frontend `AppConfig` — a lightweight loader
//! that reads `<AppConfig>/config.json` and exposes lookups the MCP
//! server needs (repo path → codeowners path, MCP settings). The
//! frontend fires an `app-config-updated` Tauri event after every save
//! so callers can call [`AppConfigStore::reload`] and get fresh state
//! without polling.

use std::{
    path::{Path, PathBuf},
    sync::RwLock,
};

use serde::{Deserialize, Serialize};

pub const DEFAULT_MCP_PORT: u16 = 47821;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RepoEntry {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "repoPath")]
    pub repo_path: String,
    #[serde(default)]
    pub codeowners: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpSettings {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_enabled() -> bool {
    true
}
fn default_port() -> u16 {
    DEFAULT_MCP_PORT
}

impl Default for McpSettings {
    fn default() -> Self {
        Self { enabled: default_enabled(), port: default_port() }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub repositories: Vec<RepoEntry>,
    #[serde(default)]
    pub mcp: McpSettings,
    // theme, etc. — deliberately ignored on the Rust side.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Thread-safe holder that can be reloaded when the frontend saves.
#[derive(Debug)]
pub struct AppConfigStore {
    config_path: PathBuf,
    inner: RwLock<AppConfig>,
}

impl AppConfigStore {
    /// Build from the Tauri `AppHandle` — resolves the app config dir the
    /// same way the frontend does (`BaseDirectory.AppConfig`).
    pub fn from_tauri(app_handle: &tauri::AppHandle) -> anyhow::Result<Self> {
        let dir = app_handle
            .path_resolver()
            .app_config_dir()
            .ok_or_else(|| anyhow::anyhow!("could not resolve app config dir"))?;
        Self::from_dir(&dir)
    }

    pub fn from_dir(app_config_dir: &Path) -> anyhow::Result<Self> {
        let config_path = app_config_dir.join("config.json");
        let inner = RwLock::new(load_from(&config_path).unwrap_or_default());
        Ok(Self { config_path, inner })
    }

    pub fn reload(&self) -> anyhow::Result<()> {
        let fresh = load_from(&self.config_path)?;
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("AppConfigStore write lock poisoned"))?;
        *guard = fresh;
        Ok(())
    }

    pub fn snapshot(&self) -> AppConfig {
        self.inner
            .read()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn mcp(&self) -> McpSettings {
        self.inner
            .read()
            .map(|g| g.mcp.clone())
            .unwrap_or_default()
    }

    /// Return the configured repo entry whose `repo_path` matches
    /// `abs_repo_path`. Path comparison is done after canonicalization
    /// so `/foo/../foo/bar` and `/foo/bar` collide.
    pub fn find_repo_by_path(&self, abs_repo_path: &Path) -> Option<RepoEntry> {
        let target = canonical(abs_repo_path);
        let guard = self.inner.read().ok()?;
        for repo in &guard.repositories {
            if canonical(Path::new(&repo.repo_path)) == target {
                return Some(repo.clone());
            }
        }
        None
    }
}

fn canonical(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

fn load_from(path: &Path) -> anyhow::Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let text = std::fs::read_to_string(path)?;
    let cfg: AppConfig = serde_json::from_str(&text)?;
    Ok(cfg)
}
