//! Tauri commands that expose MCP lifecycle, log access, and agent
//! install management to the frontend.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Listener, Manager, State};
use tokio::runtime::Handle as TokioHandle;

use crate::{
    agent_installer::{self, AgentInstallPlan, AgentKind, AgentStatus},
    app_config::AppConfigStore,
    mcp::{
        self,
        dump_store::DumpStore,
        log_store::{McpLogEntry, McpLogStore, RETENTION_DAYS},
        McpServerHandle, McpStatus,
    },
};

/// Held in Tauri managed state. Owns the log store, the AppConfig
/// mirror, the tokio runtime handle, and (optionally) the running
/// server.
pub struct McpRuntime {
    tokio: TokioHandle,
    app_config: RwLock<Option<Arc<AppConfigStore>>>,
    log_store: RwLock<Option<Arc<McpLogStore>>>,
    dump_store: RwLock<Option<Arc<DumpStore>>>,
    server: Mutex<Option<McpServerHandle>>,
    status: RwLock<McpStatus>,
}

impl McpRuntime {
    pub fn new(tokio: TokioHandle) -> Self {
        Self {
            tokio,
            app_config: RwLock::new(None),
            log_store: RwLock::new(None),
            dump_store: RwLock::new(None),
            server: Mutex::new(None),
            status: RwLock::new(McpStatus {
                running: false,
                port: 0,
                url: None,
                error: None,
            }),
        }
    }

    fn set_status(&self, status: McpStatus, app: &AppHandle) {
        if let Ok(mut s) = self.status.write() {
            *s = status.clone();
        }
        let _ = app.emit("mcp-status-changed", status);
    }

    /// Snapshot the current AppConfigStore (if initialized), for use by
    /// Tauri commands that need to resolve a repo path.
    pub fn app_config_store(&self) -> Option<Arc<AppConfigStore>> {
        self.app_config.read().ok().and_then(|g| g.clone())
    }
}

// ---- Startup wiring --------------------------------------------------

pub fn on_startup(app: AppHandle) {
    let runtime = app.state::<McpRuntime>();

    // Resolve AppConfigStore.
    let store = match AppConfigStore::from_tauri(&app) {
        Ok(s) => Arc::new(s),
        Err(err) => {
            tracing::warn!(?err, "could not load AppConfig on startup");
            return;
        }
    };
    if let Ok(mut guard) = runtime.app_config.write() {
        *guard = Some(store.clone());
    }

    // Resolve log store.
    let data_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let log = match McpLogStore::open(&data_dir) {
        Ok(s) => Arc::new(s),
        Err(err) => {
            tracing::warn!(?err, "could not open MCP log store");
            return;
        }
    };
    if let Ok(mut guard) = runtime.log_store.write() {
        *guard = Some(log.clone());
    }

    // Set up the dump store (used for truncated tool responses).
    let dump = Arc::new(DumpStore::open(&data_dir));
    if let Ok(mut guard) = runtime.dump_store.write() {
        *guard = Some(dump.clone());
    }

    // Daily prune.
    let log_for_prune = log.clone();
    runtime.tokio.spawn(async move {
        let interval = Duration::from_secs(24 * 3600);
        loop {
            tokio::time::sleep(interval).await;
            if let Err(err) = log_for_prune.prune() {
                tracing::warn!(?err, "MCP log prune failed");
            }
        }
    });

    // Listen for frontend config updates → reload AppConfigStore.
    {
        let store_clone = store.clone();
        let app_clone = app.clone();
        app.listen_any("app-config-updated", move |_event| {
            if let Err(err) = store_clone.reload() {
                tracing::warn!(?err, "failed to reload AppConfig after update event");
            }
            let _ = app_clone.emit("mcp-config-reloaded", ());
        });
    }

    // Auto-start server if enabled.
    let mcp_cfg = store.mcp();
    if mcp_cfg.enabled {
        start_server(&app, mcp_cfg.port);
    } else {
        runtime.set_status(
            McpStatus { running: false, port: mcp_cfg.port, url: None, error: None },
            &app,
        );
    }

    // Cleanup on window close. In v2 window events are delivered via
    // `WebviewWindow::on_window_event` rather than the v1
    // `tauri://close-requested` global event.
    if let Some(window) = app.get_webview_window("main") {
        let app_for_close = app.clone();
        window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                let runtime = app_for_close.state::<McpRuntime>();
                stop_server(&runtime);
            }
        });
    }
}

fn start_server(app: &AppHandle, port: u16) {
    let runtime = app.state::<McpRuntime>();
    // Take out the store + log first so we don't hold locks across
    // await.
    let store = match runtime.app_config.read().ok().and_then(|g| g.clone()) {
        Some(s) => s,
        None => return,
    };
    let log = match runtime.log_store.read().ok().and_then(|g| g.clone()) {
        Some(l) => l,
        None => return,
    };
    let dump = match runtime.dump_store.read().ok().and_then(|g| g.clone()) {
        Some(d) => d,
        None => return,
    };
    let app_for_status = app.clone();
    let handle = runtime.tokio.spawn(async move {
        match mcp::start(port, store, log, dump).await {
            Ok(handle) => {
                let runtime = app_for_status.state::<McpRuntime>();
                if let Ok(mut guard) = runtime.server.lock() {
                    *guard = Some(handle);
                }
                let status = McpStatus {
                    running: true,
                    port,
                    url: Some(format!("http://127.0.0.1:{port}/mcp")),
                    error: None,
                };
                runtime.set_status(status, &app_for_status);
            }
            Err(err) => {
                let runtime = app_for_status.state::<McpRuntime>();
                let status = McpStatus {
                    running: false,
                    port,
                    url: None,
                    error: Some(err),
                };
                runtime.set_status(status, &app_for_status);
            }
        }
    });
    // We deliberately drop the join handle — the server task lives for
    // as long as the runtime is alive.
    std::mem::drop(handle);
}

fn stop_server(runtime: &McpRuntime) {
    let handle = match runtime.server.lock() {
        Ok(mut g) => g.take(),
        Err(_) => return,
    };
    if let Some(handle) = handle {
        runtime.tokio.spawn(async move {
            handle.shutdown().await;
        });
    }
}

// ---- Tauri commands: MCP lifecycle -----------------------------------

#[tauri::command]
pub fn mcp_get_status(runtime: State<'_, McpRuntime>) -> McpStatus {
    runtime
        .status
        .read()
        .map(|s| s.clone())
        .unwrap_or(McpStatus {
            running: false,
            port: 0,
            url: None,
            error: None,
        })
}

#[tauri::command]
pub fn mcp_start(port: u16, app: AppHandle) {
    let runtime = app.state::<McpRuntime>();
    stop_server(&runtime);
    start_server(&app, port);
}

#[tauri::command]
pub fn mcp_stop(app: AppHandle) {
    let runtime = app.state::<McpRuntime>();
    stop_server(&runtime);
    let cfg_port = runtime
        .app_config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|s| s.mcp().port))
        .unwrap_or(0);
    runtime.set_status(
        McpStatus {
            running: false,
            port: cfg_port,
            url: None,
            error: None,
        },
        &app,
    );
}

#[tauri::command]
pub fn mcp_restart(port: u16, app: AppHandle) {
    let runtime = app.state::<McpRuntime>();
    stop_server(&runtime);
    start_server(&app, port);
}

#[tauri::command]
pub fn mcp_reload_config(app: AppHandle) {
    let runtime = app.state::<McpRuntime>();
    if let Some(store) = runtime.app_config.read().ok().and_then(|g| g.clone()) {
        let _ = store.reload();
    }
    let _ = app.emit("mcp-config-reloaded", ());
}

// ---- Tauri commands: log store ---------------------------------------

#[derive(Debug, Deserialize)]
pub struct LogListArgs {
    pub limit: Option<usize>,
    pub before: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct LogListResponse {
    pub entries: Vec<McpLogEntry>,
    #[serde(rename = "retentionDays")]
    pub retention_days: i64,
}

#[tauri::command]
pub fn mcp_log_list(
    args: LogListArgs,
    runtime: State<'_, McpRuntime>,
) -> LogListResponse {
    let limit = args.limit.unwrap_or(200).min(2_000);
    let entries = runtime
        .log_store
        .read()
        .ok()
        .and_then(|g| g.clone())
        .map(|log| log.list(limit, args.before))
        .unwrap_or_default();
    LogListResponse { entries, retention_days: RETENTION_DAYS }
}

#[tauri::command]
pub fn mcp_log_get(id: String, runtime: State<'_, McpRuntime>) -> Option<McpLogEntry> {
    runtime
        .log_store
        .read()
        .ok()
        .and_then(|g| g.clone())
        .and_then(|log| log.get(&id))
}

#[tauri::command]
pub fn mcp_log_clear(runtime: State<'_, McpRuntime>) -> Result<(), String> {
    let log = runtime
        .log_store
        .read()
        .ok()
        .and_then(|g| g.clone())
        .ok_or_else(|| "log store not initialized".to_string())?;
    log.clear().map_err(|e| e.to_string())
}

// ---- Tauri commands: agent install -----------------------------------

fn current_url(runtime: &McpRuntime) -> String {
    let status = runtime
        .status
        .read()
        .map(|s| s.clone())
        .unwrap_or(McpStatus {
            running: false,
            port: 0,
            url: None,
            error: None,
        });
    status
        .url
        .clone()
        .unwrap_or_else(|| format!("http://127.0.0.1:{}/mcp", status.port))
}

#[tauri::command]
pub fn agent_status(agent: AgentKind, runtime: State<'_, McpRuntime>) -> AgentStatus {
    let url = current_url(&runtime);
    agent_installer::status(agent, &url)
}

#[tauri::command]
pub fn agent_install_preview(
    agent: AgentKind,
    runtime: State<'_, McpRuntime>,
) -> AgentInstallPlan {
    let url = current_url(&runtime);
    agent_installer::plan_install(agent, &url)
}

#[tauri::command]
pub fn agent_install(
    agent: AgentKind,
    runtime: State<'_, McpRuntime>,
) -> Result<AgentInstallPlan, String> {
    let url = current_url(&runtime);
    agent_installer::install(agent, &url).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn agent_uninstall(agent: AgentKind) -> Result<(), String> {
    agent_installer::uninstall(agent).map_err(|e| e.to_string())
}
