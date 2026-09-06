//! MCP server lifecycle + the sole `get_codeowners` tool handler.
//!
//! Streamable HTTP transport over `127.0.0.1:<port>`. All I/O runs on
//! the tokio runtime we spawn ourselves so we don't block Tauri's main
//! thread.
//!
//! The tool returns a plain-text DSL body (see `dsl_emitter.rs`) rather
//! than JSON. That saves tokens on large ownership queries and is what
//! the response `Content::text(..)` carries.

pub mod compactor;
pub mod dsl_emitter;
pub mod dump_store;
pub mod error;
pub mod export_builder;
pub mod log_store;
pub mod path_expander;
pub mod port_holder;
pub mod repo_resolver;
pub mod schema;
pub mod size_guard;
pub mod tool_export_codeowners;
pub mod tool_get_codeowners;
pub mod tool_list_owners;
pub mod tool_owners_stats;

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
    dump_store::DumpStore,
    log_store::{LogStatus, McpLogEntry, McpLogStore},
    schema::{ExportCodeownersInput, GetCodeownersInput, ListOwnersInput, OwnersStatsInput},
};

pub const SERVER_INSTRUCTIONS: &str = concat!(
    "codeowners-viewer exposes four tools:\n",
    "  - `get_codeowners` — returns CODEOWNERS-derived ownership for ",
    "files in a git repo as a plain-text DSL. Use for direct reads.\n",
    "  - `export_codeowners` — dumps the full ownership map for a repo ",
    "to a temp JSON file. Returns just the file path + counts. Use ",
    "when you plan to run a script (e.g. Python) against the data.\n",
    "  - `list_owners` — returns the distinct owner handles present in ",
    "the repo at HEAD. Use for discovery before filtering.\n",
    "  - `owners_stats` — returns per-owner file counts + repo-wide ",
    "totals at HEAD. Use to rank owners by footprint.\n\n",
    "===== get_codeowners =====\n\n",
    "Inputs: ",
    "{ repo (absolute path), for (\"branch\" | \"changed_files\"), ",
    "branch? (defaults to \"HEAD\"), paths? (repo-root-relative files, ",
    "directories, or globs; mixed OK), responseMode (\"compact\" default | ",
    "\"full\"), maxDepth? (integer; when set, subtrees deeper than this ",
    "many levels below each entry of `paths` are collapsed — see the ",
    "TRUNCATED note below. Depth 0 = the requested path itself; depth 1 ",
    "= its immediate children. Ignored when `paths` is empty.) }.\n\n",
    "Output is a plain-text DSL (not JSON). Grammar:\n",
    "Header (until first blank line):\n",
    "  format: codeowners-dsl-v1\n",
    "  base: <common dir prefix stripped from every tree path>\n",
    "  default: <rule id that applies to anything not otherwise listed>\n",
    "  truncated: true|false\n",
    "  sizeBytes: <byte length of this response>\n",
    "  fullDumpPath: <path>   # only when truncated=true\n\n",
    "rules: section — one line per referenced rule:\n",
    "  <line-number> <owner> [<owner> ...] [(<comment>)]\n",
    "  Only tokens starting with `@` are owners; the FIRST owner is the ",
    "one on the underlying CODEOWNERS rule and the ones after it are ",
    "additional owners on that same rule (co-owners). The parenthesized ",
    "`(<comment>)` is a VERBATIM passthrough of the trailing `# ...` ",
    "comment on the CODEOWNERS line, with the leading `#` stripped. Its ",
    "contents are NOT part of the DSL grammar — they may contain ",
    "project-specific conventions (e.g. `!required` markers, extra ",
    "`@owner` mentions used as notes, free-form English text). Treat ",
    "the comment as informational only; do NOT parse `@owner` tokens ",
    "inside a comment as additional owners. `)` inside comments is ",
    "escaped as `\\)`. A bare `<line-number>` (no owners, `(unowned)` ",
    "comment) is an unowned rule.\n\n",
    "Tree body: 1-space indent per level.\n",
    "  <dir>/[ <rule-id> | [id:count,...] TRUNCATED]\n",
    "  <file>[ <rule-id>]\n",
    "Rules:\n",
    "  - Trailing `/` marks a directory.\n",
    "  - A rule id on a directory line sets that subtree's default.\n",
    "  - A file or dir with no rule id inherits the nearest ancestor's ",
    "    default (root's `default:` if none override in between).\n",
    "  - `[id:count,...] TRUNCATED` means the subtree was collapsed ",
    "    (either by `maxDepth` or the 50 KB size guard). Each entry is a ",
    "    rule id and the number of files it owns inside the collapsed ",
    "    subtree; sum of counts = total files inside. Entries are sorted ",
    "    by descending count (tiebreak: ascending rule id). Marker is ",
    "    only emitted when the collapsed subtree mixes ≥ 2 rules — a ",
    "    single-rule collapse just renders as `dir/ <rule>` since no ",
    "    information is lost. The full untruncated body is at ",
    "    `fullDumpPath` when the size guard fires; re-request the ",
    "    subtree with a larger `maxDepth` (or narrower `paths`) to see ",
    "    it inline.\n",
    "Response modes:\n",
    "  - compact (default): omit files whose rule matches the enclosing ",
    "    default; directory overrides are emitted where they reduce ",
    "    noise.\n",
    "  - full: every file the caller asked about is listed with its ",
    "    rule id.\n",
    "When `for = \"changed_files\"` the `paths` list is additive — omit ",
    "it for just the working-tree changed set.\n\n",
    "===== export_codeowners =====\n\n",
    "Inputs: { repo (absolute path), owners? (string[] — filter files ",
    "by owner; OR within the list), extensions? (string[] — filter by ",
    "extension, case-insensitive, leading `.` ignored; OR within the ",
    "list), path? (absolute path to write the dump to; overwrites if ",
    "the file already exists; parent directories are created; when ",
    "omitted, a fresh file is written under the OS temp dir and stale ",
    "dumps are pruned). Filters combine as AND. Always full-repo dump ",
    "at HEAD. No `paths`, no `maxDepth`, no `responseMode`. }\n\n",
    "Output: JSON with { path, sizeBytes, fileCount, ruleCount, schema }. ",
    "The `path` points to a temp file on disk containing the actual data ",
    "— read it yourself (e.g. with `open(path)` in Python). Files ",
    "matching `export-*.json` older than 7 days in the temp dir are ",
    "pruned on each invocation.\n\n",
    "Dump file schema (`codeowners-export/v1`):\n",
    "{\n",
    "  \"schema\": \"codeowners-export/v1\",\n",
    "  \"rules\": {\n",
    "    \"42\": {\"owners\": [\"@fivetran/kepler\"], \"comment\": \"!required\"},\n",
    "    \"118\": {\"owners\": [\"@fivetran/bacon\", \"@fivetran/korolev\"], \"comment\": null},\n",
    "    \"0\": {\"owners\": [], \"comment\": null}\n",
    "  },\n",
    "  \"files\": {\n",
    "    \"app/src/Main.java\": 42,\n",
    "    \"app/misc/orphan.txt\": 0\n",
    "  },\n",
    "  \"stats\": {\n",
    "    \"total_files\": 41203,\n",
    "    \"owned_files\": 41198,\n",
    "    \"unowned_files\": 5,\n",
    "    \"rule_count\": 187,\n",
    "    \"codeowners_file_path\": \".github/CODEOWNERS\",\n",
    "    \"filtered\": false,\n",
    "    \"generated_at\": \"2026-09-06T18:15:28Z\"\n",
    "  }\n",
    "}\n",
    "Semantics:\n",
    "  - Rule keys are 1-based CODEOWNERS line numbers, as strings. Same ",
    "    vocabulary as the `get_codeowners` DSL.\n",
    "  - Rule `\"0\"` is a reserved sentinel meaning \"no matching ",
    "    CODEOWNERS rule\". Its `owners` is []. Only present when at ",
    "    least one unowned file remains post-filter.\n",
    "  - `files` values are the rule id for that path. Paths are ",
    "    relative to repo root — no base stripping.\n",
    "  - `comment` is the verbatim inline `# ...` comment from CODEOWNERS ",
    "    with the leading `#` and whitespace stripped, or `null` if the ",
    "    rule had no trailing comment. May contain project-specific ",
    "    markers such as `!required`.\n",
    "  - `stats.filtered` is true iff the input `owners` or `extensions` ",
    "    filter was non-empty. When filtered, `rules` only holds rules ",
    "    actually referenced by the remaining files.\n",
    "  - `stats.generated_at` is an ISO-8601 UTC timestamp (second ",
    "    precision, trailing `Z`).\n\n",
    "===== list_owners =====\n\n",
    "Inputs: { repo (absolute path) }.\n\n",
    "Output: JSON `{ \"owners\": [\"@team/a\", \"@team/b\", ...] }`. ",
    "Distinct owner handles present in CODEOWNERS-matched files at HEAD, ",
    "alphabetically sorted. Unowned files do NOT contribute an entry.\n\n",
    "===== owners_stats =====\n\n",
    "Inputs: { repo (absolute path) }.\n\n",
    "Output: JSON `{ totalFiles, unownedFiles, owners: { \"@team/a\": {files: N}, ... } }` ",
    "with owners alphabetically sorted by handle. Wrap the count in a ",
    "`{files: N}` object (not a raw number) so the shape stays ",
    "extensible. A file with N co-owners contributes +1 to each of ",
    "those N owners, so the sum of per-owner `files` may exceed ",
    "`totalFiles`. Unowned files are counted in `unownedFiles` but not ",
    "attributed to any owner.",
);

// ---- Handler ----------------------------------------------------------

#[derive(Clone)]
pub struct CodeownersMcp {
    app_config: Arc<AppConfigStore>,
    log_store: Arc<McpLogStore>,
    dump_store: Arc<DumpStore>,
    tool_router: ToolRouter<CodeownersMcp>,
}

impl CodeownersMcp {
    fn new(
        app_config: Arc<AppConfigStore>,
        log_store: Arc<McpLogStore>,
        dump_store: Arc<DumpStore>,
    ) -> Self {
        Self {
            app_config,
            log_store,
            dump_store,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl CodeownersMcp {
    #[tool(
        description = "Return CODEOWNERS-derived ownership for files in a repo as a compact DSL (not JSON). \
Inputs: `repo` (absolute path), `for` (\"branch\" or \"changed_files\"), \
`branch?` (defaults to \"HEAD\"), `paths?` (repo-root-relative — files, \
directories (recursive), or globs; mixed OK), `responseMode` \
(\"compact\" default | \"full\"). `paths` is additive when `for = \
\"changed_files\"`. See the server instructions for the DSL grammar."
    )]
    async fn get_codeowners(
        &self,
        Parameters(input): Parameters<GetCodeownersInput>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let request_json = serde_json::to_value(&input).unwrap_or(serde_json::Value::Null);

        let result = tool_get_codeowners::run(
            &self.app_config,
            Some(&self.dump_store),
            input,
        );

        let elapsed_ms = started.elapsed().as_millis() as u64;
        let (status, response_json, response_size, tool_result) = match result {
            Ok(out) => {
                let size = out.body.len() as u64;
                (
                    LogStatus::Ok,
                    serde_json::Value::String(out.body.clone()),
                    Some(size),
                    CallToolResult::success(vec![Content::text(out.body)]),
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
                let size = text.len() as u64;
                (
                    LogStatus::Error,
                    json,
                    Some(size),
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
            response_size_bytes: response_size,
        };
        self.log_store.append(&entry);

        Ok(tool_result)
    }

    #[tool(
        description = "Dump the full CODEOWNERS-derived ownership map for a repo to \
a JSON file and return its path + counts. Use when you need to run \
a script (e.g. Python) over the data — the JSON stdlib parses it \
directly. For direct reads, use `get_codeowners` instead. Inputs: \
`repo` (absolute path), `owners?` (string[]; filter files by owner, OR \
within the list), `extensions?` (string[]; filter by extension, \
case-insensitive; OR within the list), `path?` (absolute path where \
the dump should be written; overwrites any existing file; parent \
dirs are created; defaults to a fresh file under the OS temp dir). \
Both filters combine as AND. Always full-repo at HEAD. See server \
instructions for the JSON schema."
    )]
    async fn export_codeowners(
        &self,
        Parameters(input): Parameters<ExportCodeownersInput>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let request_json = serde_json::to_value(&input).unwrap_or(serde_json::Value::Null);

        let result = tool_export_codeowners::run(&self.app_config, input);

        let elapsed_ms = started.elapsed().as_millis() as u64;
        let (status, response_json, response_size, tool_result) = match result {
            Ok(out) => {
                let json = serde_json::to_value(&out).unwrap_or(serde_json::Value::Null);
                let text = serde_json::to_string(&out)
                    .unwrap_or_else(|_| "{}".to_string());
                let size = text.len() as u64;
                (
                    LogStatus::Ok,
                    json,
                    Some(size),
                    CallToolResult::success(vec![Content::text(text)]),
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
                let size = text.len() as u64;
                (
                    LogStatus::Error,
                    json,
                    Some(size),
                    CallToolResult::error(vec![Content::text(text)]),
                )
            }
        };

        let entry = McpLogEntry {
            id: gen_id(),
            timestamp: Utc::now(),
            tool: "export_codeowners".to_string(),
            request: request_json,
            response: response_json,
            duration_ms: elapsed_ms,
            status,
            response_size_bytes: response_size,
        };
        self.log_store.append(&entry);

        Ok(tool_result)
    }

    #[tool(
        description = "Return the distinct CODEOWNERS owner handles present in the repo \
at HEAD, alphabetically sorted. Use this to discover the space of valid \
owner strings before calling `export_codeowners` with an `owners[]` \
filter. Inputs: `repo` (absolute path). Output: JSON \
`{ \"owners\": [\"@team/a\", \"@team/b\", ...] }`. Unowned files do NOT \
contribute an entry."
    )]
    async fn list_owners(
        &self,
        Parameters(input): Parameters<ListOwnersInput>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let request_json = serde_json::to_value(&input).unwrap_or(serde_json::Value::Null);

        let result = tool_list_owners::run(&self.app_config, input);

        let elapsed_ms = started.elapsed().as_millis() as u64;
        let (status, response_json, response_size, tool_result) = match result {
            Ok(out) => {
                let json = serde_json::to_value(&out).unwrap_or(serde_json::Value::Null);
                let text = serde_json::to_string(&out)
                    .unwrap_or_else(|_| "{}".to_string());
                let size = text.len() as u64;
                (
                    LogStatus::Ok,
                    json,
                    Some(size),
                    CallToolResult::success(vec![Content::text(text)]),
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
                let size = text.len() as u64;
                (
                    LogStatus::Error,
                    json,
                    Some(size),
                    CallToolResult::error(vec![Content::text(text)]),
                )
            }
        };

        let entry = McpLogEntry {
            id: gen_id(),
            timestamp: Utc::now(),
            tool: "list_owners".to_string(),
            request: request_json,
            response: response_json,
            duration_ms: elapsed_ms,
            status,
            response_size_bytes: response_size,
        };
        self.log_store.append(&entry);

        Ok(tool_result)
    }

    #[tool(
        description = "Return per-owner file counts + repo-wide totals for the repo \
at HEAD. Use to rank owners by footprint before picking a filter for \
`export_codeowners`. Inputs: `repo` (absolute path). Output: JSON \
`{ totalFiles, unownedFiles, owners: { \"@team/a\": {files: N}, ... } }` \
alphabetically sorted by owner handle. A file with N co-owners \
contributes +1 to each of those N owners, so the sum of per-owner \
`files` may exceed `totalFiles`. Unowned files are counted in \
`unownedFiles` but not attributed to any owner."
    )]
    async fn owners_stats(
        &self,
        Parameters(input): Parameters<OwnersStatsInput>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let request_json = serde_json::to_value(&input).unwrap_or(serde_json::Value::Null);

        let result = tool_owners_stats::run(&self.app_config, input);

        let elapsed_ms = started.elapsed().as_millis() as u64;
        let (status, response_json, response_size, tool_result) = match result {
            Ok(out) => {
                let json = serde_json::to_value(&out).unwrap_or(serde_json::Value::Null);
                let text = serde_json::to_string(&out)
                    .unwrap_or_else(|_| "{}".to_string());
                let size = text.len() as u64;
                (
                    LogStatus::Ok,
                    json,
                    Some(size),
                    CallToolResult::success(vec![Content::text(text)]),
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
                let size = text.len() as u64;
                (
                    LogStatus::Error,
                    json,
                    Some(size),
                    CallToolResult::error(vec![Content::text(text)]),
                )
            }
        };

        let entry = McpLogEntry {
            id: gen_id(),
            timestamp: Utc::now(),
            tool: "owners_stats".to_string(),
            request: request_json,
            response: response_json,
            duration_ms: elapsed_ms,
            status,
            response_size_bytes: response_size,
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
    dump_store: Arc<DumpStore>,
) -> Result<McpServerHandle, String> {
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let listener = TcpListener::bind(addr).await.map_err(|e| {
        let base = format!("could not bind {addr}: {e}");
        if e.kind() == std::io::ErrorKind::AddrInUse {
            if let Some(holder) = port_holder::describe_listener(port) {
                return format!("{base}. Port is held by {holder}.");
            }
        }
        base
    })?;

    let handler_factory_app = app_config.clone();
    let handler_factory_log = log_store.clone();
    let handler_factory_dump = dump_store.clone();

    let service = StreamableHttpService::new(
        move || {
            Ok(CodeownersMcp::new(
                handler_factory_app.clone(),
                handler_factory_log.clone(),
                handler_factory_dump.clone(),
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
