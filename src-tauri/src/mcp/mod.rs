//! MCP server lifecycle + the single `get_codeowners` tool handler.
//!
//! Streamable HTTP transport over `127.0.0.1:<port>`. All I/O is done
//! on the tokio runtime we spawn ourselves so we don't block Tauri's
//! main thread.

pub mod compactor;
pub mod error;
pub mod log_store;
pub mod path_expander;
pub mod repo_resolver;
pub mod schema;
pub mod tool_get_codeowners;

use std::{
    net::SocketAddr,
    sync::Arc,
    time::Instant,
};

use axum::Router;
use chrono::Utc;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion,
        ServerCapabilities, ServerInfo,
    },
    tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig,
        StreamableHttpService,
    },
    ErrorData as McpError, ServerHandler,
};
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

use crate::app_config::AppConfigStore;

use self::{
    log_store::{LogStatus, McpLogEntry, McpLogStore},
    schema::GetCodeownersInput,
};

pub const SERVER_INSTRUCTIONS: &str = concat!(
    "codeowners-viewer exposes a single tool `get_codeowners` that returns ",
    "CODEOWNERS-derived ownership information for files in a git repo. ",
    "Inputs: { repo (absolute path), for (\"branch\" | \"changed_files\"), ",
    "branch? (defaults to \"HEAD\"), paths? (repo-root-relative files, ",
    "directories, or globs — mixed types are OK), responseMode ",
    "(\"compact\" | \"normal\" | \"full\") }. Compact groups by owner-set ",
    "and collapses directories where every descendant shares the same ",
    "owners. Normal returns one entry per file. Full adds the CODEOWNERS ",
    "line number of the matching rule. When `for = \"changed_files\"` the ",
    "`paths` list is additive — omit it for just the changed set.",
);

// ---- Handler ----------------------------------------------------------

#[derive(Clone)]
pub struct CodeownersMcp {
    app_config: Arc<AppConfigStore>,
    log_store: Arc<McpLogStore>,
    tool_router: ToolRouter<CodeownersMcp>,
}

impl CodeownersMcp {
    fn new(app_config: Arc<AppConfigStore>, log_store: Arc<McpLogStore>) -> Self {
        Self {
            app_config,
            log_store,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl CodeownersMcp {
    #[tool(
        description = "Return CODEOWNERS-derived ownership for files in a repo. \
Inputs: `repo` (absolute path), `for` (\"branch\" or \"changed_files\"), \
`branch?` (defaults to \"HEAD\"), `paths?` (repo-root-relative — files, \
directories (recursive), or globs; mixed OK), `responseMode` \
(\"compact\" | \"normal\" | \"full\"). `paths` is additive when `for = \
\"changed_files\"`."
    )]
    async fn get_codeowners(
        &self,
        Parameters(input): Parameters<GetCodeownersInput>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let request_json = serde_json::to_value(&input).unwrap_or(serde_json::Value::Null);

        let result = tool_get_codeowners::run(&self.app_config, input);

        let elapsed_ms = started.elapsed().as_millis() as u64;
        let (status, response_json, tool_result) = match result {
            Ok(out) => {
                let json = serde_json::to_value(&out)
                    .unwrap_or(serde_json::Value::Null);
                let body = serde_json::to_string(&out)
                    .unwrap_or_else(|_| "{}".to_string());
                (
                    LogStatus::Ok,
                    json,
                    CallToolResult::success(vec![Content::text(body)]),
                )
            }
            Err(err) => {
                let payload = ToolErrorPayload {
                    error: err.clone(),
                    message: err.to_string(),
                };
                let json = serde_json::to_value(&payload)
                    .unwrap_or(serde_json::Value::Null);
                let text = serde_json::to_string(&payload)
                    .unwrap_or_else(|_| err.to_string());
                (
                    LogStatus::Error,
                    json,
                    CallToolResult::error(vec![Content::text(text)]),
                )
            }
        };

        let entry = McpLogEntry {
            id: gen_id(),
            timestamp: Utc::now(),
            tool: "get_codeowners".to_string(),
            request: request_json,
            response: response_json,
            duration_ms: elapsed_ms,
            status,
        };
        self.log_store.append(&entry);

        Ok(tool_result)
    }
}

#[tool_handler]
impl ServerHandler for CodeownersMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(SERVER_INSTRUCTIONS.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ToolErrorPayload {
    error: error::McpToolError,
    message: String,
}

// ---- Lifecycle --------------------------------------------------------

/// Snapshot of server state exposed to Tauri commands / UI polling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStatus {
    pub running: bool,
    pub port: u16,
    pub url: Option<String>,
    pub error: Option<String>,
}

pub struct McpServerHandle {
    pub port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
    join: Option<JoinHandle<()>>,
}

impl McpServerHandle {
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.join.take() {
            let _ = handle.await;
        }
    }
}

/// Try to bind + start the server on `127.0.0.1:port`. Returns an
/// `McpServerHandle` on success or a `String` error message on bind
/// failure (e.g. address in use) — surfaced verbatim to the UI.
pub async fn start(
    port: u16,
    app_config: Arc<AppConfigStore>,
    log_store: Arc<McpLogStore>,
) -> Result<McpServerHandle, String> {
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("could not bind {addr}: {e}"))?;

    let handler_factory_app = app_config.clone();
    let handler_factory_log = log_store.clone();

    let service = StreamableHttpService::new(
        move || {
            Ok(CodeownersMcp::new(
                handler_factory_app.clone(),
                handler_factory_log.clone(),
            ))
        },
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );

    let router = Router::new().nest_service("/mcp", service);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let join = tokio::spawn(async move {
        let serve = axum::serve(listener, router).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });
        if let Err(err) = serve.await {
            tracing::warn!(?err, "MCP HTTP server exited with error");
        }
    });

    Ok(McpServerHandle {
        port,
        shutdown_tx: Some(shutdown_tx),
        join: Some(join),
    })
}

fn gen_id() -> String {
    use rand::RngCore;
    let mut buf = [0u8; 12];
    rand::rng().fill_bytes(&mut buf);
    hex_encode(&buf)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}
