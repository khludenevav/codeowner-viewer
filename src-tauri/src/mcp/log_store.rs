//! JSONL-backed request/response log for MCP tool calls.
//!
//! One line per invocation. Kept for 7 days; pruned on server start and
//! once every 24 h afterwards. Also capped at 5000 rows so a chatty
//! session can't blow up the file.

use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const RETENTION_DAYS: i64 = 7;
pub const MAX_ROWS: usize = 5_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpLogEntry {
    pub id: String,
    #[serde(rename = "ts")]
    pub timestamp: DateTime<Utc>,
    pub tool: String,
    /// Serialized JSON of the request body. Kept as a `Value` to allow
    /// any shape.
    pub request: serde_json::Value,
    pub response: serde_json::Value,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
    pub status: LogStatus,
    /// Byte size of the tool's serialized response body, measured after
    /// the DSL emitter runs (or the error payload's serialized length
    /// for failed calls). Surfaces in the UI so we can watch how well
    /// the DSL + size guard compress large repos.
    #[serde(
        rename = "responseSizeBytes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub response_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogStatus {
    Ok,
    Error,
}

/// Thread-safe append-only writer + prune helper.
#[derive(Debug)]
pub struct McpLogStore {
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl McpLogStore {
    pub fn open(app_data_dir: &Path) -> anyhow::Result<Self> {
        std::fs::create_dir_all(app_data_dir).ok();
        let path = app_data_dir.join("mcp-log.jsonl");
        let store = Self { path, write_lock: Mutex::new(()) };
        // Prune on open so the first UI read sees fresh data.
        let _ = store.prune();
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append an entry. Any I/O error is swallowed after logging — log
    /// failures must not surface as tool failures.
    pub fn append(&self, entry: &McpLogEntry) {
        let _lock = match self.write_lock.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let json = match serde_json::to_string(entry) {
            Ok(s) => s,
            Err(err) => {
                tracing::warn!(?err, "failed to serialize MCP log entry");
                return;
            }
        };
        let mut file = match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            Ok(f) => f,
            Err(err) => {
                tracing::warn!(?err, "failed to open MCP log for append");
                return;
            }
        };
        let _ = writeln!(file, "{}", json);
    }

    /// Return the most recent `limit` entries in reverse-chronological
    /// order. `before` optionally excludes entries newer than the given
    /// timestamp for basic pagination.
    pub fn list(
        &self,
        limit: usize,
        before: Option<DateTime<Utc>>,
    ) -> Vec<McpLogEntry> {
        let entries = self.read_all().unwrap_or_default();
        let mut it: Vec<McpLogEntry> = entries
            .into_iter()
            .filter(|e| before.map_or(true, |b| e.timestamp < b))
            .collect();
        it.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        it.truncate(limit);
        it
    }

    pub fn get(&self, id: &str) -> Option<McpLogEntry> {
        self.read_all()
            .ok()?
            .into_iter()
            .find(|e| e.id == id)
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        let _lock = self
            .write_lock
            .lock()
            .map_err(|_| anyhow::anyhow!("write lock poisoned"))?;
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        Ok(())
    }

    /// Drop rows older than `RETENTION_DAYS` and cap at `MAX_ROWS`.
    /// Called once at startup and by the daily prune loop.
    pub fn prune(&self) -> anyhow::Result<()> {
        let _lock = self
            .write_lock
            .lock()
            .map_err(|_| anyhow::anyhow!("write lock poisoned"))?;
        if !self.path.exists() {
            return Ok(());
        }
        let mut entries = read_all_from(&self.path).unwrap_or_default();
        let cutoff = Utc::now() - chrono::Duration::days(RETENTION_DAYS);
        entries.retain(|e| e.timestamp >= cutoff);
        entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        if entries.len() > MAX_ROWS {
            let drop_count = entries.len() - MAX_ROWS;
            entries.drain(..drop_count);
        }
        let mut tmp = self.path.clone();
        tmp.set_extension("jsonl.tmp");
        {
            let mut f = File::create(&tmp)?;
            for e in &entries {
                let s = serde_json::to_string(e)?;
                writeln!(f, "{}", s)?;
            }
        }
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    fn read_all(&self) -> anyhow::Result<Vec<McpLogEntry>> {
        read_all_from(&self.path)
    }
}

fn read_all_from(path: &Path) -> anyhow::Result<Vec<McpLogEntry>> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(err) => return Err(err.into()),
    };
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<McpLogEntry>(&line) {
            out.push(entry);
        }
    }
    Ok(out)
}

/// Spawn a periodic prune every `interval`. The task exits when the
/// returned `JoinHandle` is dropped and the runtime shuts down.
pub fn spawn_daily_prune(
    store: std::sync::Arc<McpLogStore>,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            if let Err(err) = store.prune() {
                tracing::warn!(?err, "MCP log prune failed");
            }
        }
    })
}
