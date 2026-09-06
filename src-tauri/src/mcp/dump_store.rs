//! Where truncated MCP responses go for post-hoc inspection.
//!
//! Each time the `size_guard` fires, we dump the untruncated DSL body
//! to `<app-data>/mcp-dumps/<utc>-<rand>.txt`, then reference that path
//! from the response header (`fullDumpPath:`). Dumps are pruned on
//! server start using the same 7-day cutoff as the request log so we
//! don't grow the app-data dir indefinitely.

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use chrono::{Duration, Utc};

use super::log_store::RETENTION_DAYS;

pub const DUMP_DIR_NAME: &str = "mcp-dumps";

pub struct DumpStore {
    dir: PathBuf,
}

impl DumpStore {
    pub fn open(app_data_dir: &Path) -> Self {
        let dir = app_data_dir.join(DUMP_DIR_NAME);
        let _ = fs::create_dir_all(&dir);
        let store = Self { dir };
        let _ = store.prune();
        store
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Write `body` to a freshly minted file under the dump directory
    /// and return the absolute path. On I/O error returns `None` — the
    /// caller can still emit the response body, just without the
    /// `fullDumpPath:` reference.
    pub fn write(&self, body: &str) -> Option<PathBuf> {
        let filename = new_filename();
        let path = self.dir.join(filename);
        fs::write(&path, body).ok()?;
        Some(path)
    }

    /// Drop dump files older than `RETENTION_DAYS`.
    pub fn prune(&self) -> std::io::Result<()> {
        let cutoff = Utc::now() - Duration::days(RETENTION_DAYS);
        let entries = match fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(err),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let modified: SystemTime = match entry.metadata().and_then(|m| m.modified()) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let dt: chrono::DateTime<Utc> = modified.into();
            if dt < cutoff {
                let _ = fs::remove_file(path);
            }
        }
        Ok(())
    }
}

fn new_filename() -> String {
    use rand::RngCore;
    let ts = Utc::now().format("%Y%m%dT%H%M%SZ");
    let mut buf = [0u8; 4];
    rand::rng().fill_bytes(&mut buf);
    let suffix: String = buf.iter().map(|b| format!("{:02x}", b)).collect();
    format!("{ts}-{suffix}.txt")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_creates_file_with_body() {
        let dir = TempDir::new().unwrap();
        let store = DumpStore::open(dir.path());
        let path = store.write("hello world").unwrap();
        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello world");
    }
}
