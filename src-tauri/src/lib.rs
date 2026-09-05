use std::{
    collections::HashMap,
    path::Path,
    sync::{
        atomic::{AtomicU32, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

use ahash::AHashMap;
use rayon::prelude::*;

pub mod agent_installer;
pub mod app_config;
pub mod codeowners_engine;
pub mod codeowners_file_parser;
pub mod mcp;
pub mod mcp_commands;
use serde::ser::{SerializeStruct, Serializer};
use serde::Serialize;
use tauri::Emitter;

extern crate pretty_assertions;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime for MCP");
    let rt_handle = rt.handle().clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(mcp_commands::McpRuntime::new(rt_handle))
        // Keep the runtime alive for as long as the app is running.
        .manage(rt)
        .setup(|app| {
            mcp_commands::on_startup(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_branch_files,
            get_all_codeowners_for_branch,
            get_changed_codeowners_for_branch,
            get_codeowners_for_branch_file,
            mcp_commands::mcp_get_status,
            mcp_commands::mcp_start,
            mcp_commands::mcp_stop,
            mcp_commands::mcp_restart,
            mcp_commands::mcp_log_list,
            mcp_commands::mcp_log_get,
            mcp_commands::mcp_log_clear,
            mcp_commands::mcp_reload_config,
            mcp_commands::agent_status,
            mcp_commands::agent_install_preview,
            mcp_commands::agent_install,
            mcp_commands::agent_uninstall,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/** Return all files in the repo for passed branch */
#[tauri::command(async)]
fn get_branch_files(abs_repo_path: &str, branch: &str) -> String {
    let files = get_branch_files_vector(abs_repo_path, branch);
    serde_json::to_string(&files).unwrap()
}

/** Return all files in the repo for passed branch */
#[tauri::command(async)]
fn get_all_codeowners_for_branch(
    app_handle: tauri::AppHandle,
    abs_repo_path: &str,
    branch: &str,
    session_id: &str,
) -> String {
    let all_owners =
        get_all_codeowners_for_branch_struct(app_handle, abs_repo_path, branch, session_id);
    serde_json::to_string(&all_owners).unwrap()
}

/** Returns comments for codeowners file of passed branch */
#[tauri::command(async)]
fn get_codeowners_for_branch_file(abs_repo_path: &str, branch: &str, file: &str) -> String {
    let codeowners_content = get_codeowners_content(abs_repo_path, branch);
    let codeowners = codeowners_file_parser::from_reader(codeowners_content.as_bytes());
    get_joined_codeowners(codeowners.of(file)).unwrap_or(String::from(""))
}

/** Key is team or empty, value is changed files for branch */
#[tauri::command(async)]
fn get_changed_codeowners_for_branch(abs_repo_path: &str, branch: &str) -> String {
    let codeowners_content = get_codeowners_content(abs_repo_path, branch);
    let codeowners = codeowners_file_parser::from_reader(codeowners_content.as_bytes());
    let branch_diff = get_branch_diff(abs_repo_path, branch);

    let mut owners_dictionary: HashMap<String, Vec<FrontendFile>> = HashMap::new();
    for file_path in branch_diff.split("\n") {
        if file_path.is_empty() {
            // it is for latest line
            continue;
        }
        // Combined lookup: one pattern scan yields both owners and the
        // rule's inline comment. Halves the matching work compared
        // to calling `of()` + `comment_of()` back-to-back.
        //
        // Parser records every inline `#…` comment now (broadened for the
        // MCP tool); the existing UI historically only surfaced
        // `#!…`-style directives, so we keep that filter here.
        let (owners_opt, comment_opt) = codeowners.of_with_comment(file_path);
        let owner_team = get_joined_codeowners(owners_opt);
        let comment = comment_opt
            .filter(|s| s.starts_with("#!"))
            .map(|s| s.to_string());
        let frontend_file = FrontendFile { path: file_path.to_string(), comment };

        owners_dictionary
            .entry(owner_team.unwrap_or(String::new()))
            .and_modify(|e| e.push(FrontendFile { path: frontend_file.path.clone(), comment: frontend_file.comment.clone() }))
            .or_insert(vec![frontend_file]);
    }
    let mut result: Vec<FrontendCodeowner> = owners_dictionary
        .into_iter()
        .map(|(owners, files)| FrontendCodeowner { owners, files })
        .collect::<Vec<FrontendCodeowner>>();
    // We have to send stable data
    result.sort_by(|a, b| a.owners.cmp(&b.owners));
    serde_json::to_string(&result).unwrap()
}

/** Returns difference as list of changed files between passed branch and main */
fn get_branch_diff(abs_repo_path: &str, branch: &str) -> String {
    codeowners_engine::get_branch_diff(abs_repo_path, branch)
}

/** Return all files in the repo for passed branch */
fn get_branch_files_vector(abs_repo_path: &str, branch: &str) -> Vec<String> {
    codeowners_engine::get_branch_files_vector(abs_repo_path, branch)
}

/** Returns comments for codeowners file of passed branch */
fn get_codeowners_content(abs_repo_path: &str, branch: &str) -> String {
    codeowners_engine::get_codeowners_content_at_ref(abs_repo_path, branch, "CODEOWNERS")
}

fn get_joined_codeowners(
    owners_vec: Option<&Vec<codeowners_file_parser::Owner>>,
) -> Option<String> {
    match owners_vec {
        None => None,
        Some(owners) => Some(
            owners
                .iter()
                .map(|owner| format!("{owner}"))
                .collect::<Vec<String>>()
                .join(", "),
        ),
    }
}

#[derive(Serialize, Clone)]
struct AllCodeownersProgressPayload {
    files_handled: u32,
    files_total: u32,
    session_id: String,
    abs_repo_path: String,
    branch: String,
}

#[derive(Serialize)]
struct FrontendFile {
    path: String,
    comment: Option<String>,
}

struct FrontendCodeowner {
    owners: String,
    files: Vec<FrontendFile>,
}

impl Serialize for FrontendCodeowner {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("FrontendCodeowner", 2)?;
        state.serialize_field("owners", &self.owners)?;
        state.serialize_field("files", &self.files)?;
        state.end()
    }
}

fn get_all_codeowners_for_branch_struct(
    app_handle: tauri::AppHandle,
    abs_repo_path: &str,
    branch: &str,
    session_id: &str,
) -> DirectoryOwners {
    let codeowners_content = get_codeowners_content(abs_repo_path, branch);
    let codeowners = codeowners_file_parser::from_reader(codeowners_content.as_bytes());
    let files = get_branch_files_vector(abs_repo_path, branch);
    let owner_strings = codeowners.owner_strings();
    let files_total = files.len() as u32;

    // Emit `0 / total` **immediately** so the UI can leave its "0 / 0"
    // placeholder state as soon as the file list is known — before the
    // (possibly expensive) ancestor-cache build phase runs. On big repos
    // this is the difference between the UI staring at 0/0 for many
    // seconds and it showing "0 / 130000, working…" right away.
    let emit_payload = |handled: u32| {
        let payload = AllCodeownersProgressPayload {
            files_handled: handled,
            files_total,
            session_id: session_id.to_string(),
            abs_repo_path: abs_repo_path.to_string(),
            branch: branch.to_string(),
        };
        app_handle
            .emit("all-codeowners-progress", payload)
            .unwrap();
    };
    emit_payload(0);

    // -------------------------------------------------------------------
    // Phase 1: build the ancestor-directory memoization cache.
    //
    // For every unique parent directory that appears in `files`, precompute
    // "first rule index that direct-matches this directory or any of its
    // ancestors, ignoring rules that end with `/*`". This is the shared
    // parent-walk piece of `Owners::of` and used to dominate the total cost
    // when many files share the same directory.
    //
    // The direct-match probe for a single directory is the expensive part
    // (it scans up to P patterns), so we parallelize it across the unique
    // directories. The final combine-with-parent pass is O(unique dirs)
    // integer work and runs serially.
    // -------------------------------------------------------------------
    let ancestor_cache = codeowners_engine::build_ancestor_cache(&codeowners, &files);

    // -------------------------------------------------------------------
    // Phase 2: resolve every file in parallel using the cache.
    //
    // Rayon's `par_chunks` preserves input order in the collected result,
    // so the downstream tree assembly is deterministic and matches
    // pre-parallel behavior (`git ls-tree` order).
    // -------------------------------------------------------------------
    let counter = AtomicU32::new(0);
    let last_emit = Mutex::new(Instant::now());

    // Chunk size is picked to keep IPC-throttled progress events smooth
    // while amortizing the per-chunk atomic + Mutex work.
    let num_threads = rayon::current_num_threads().max(1);
    let chunk_size = 512usize.max(files.len() / (num_threads * 8).max(1));

    let owner_indices: Vec<Option<usize>> = files
        .par_chunks(chunk_size)
        .flat_map_iter(|chunk| {
            let out: Vec<Option<usize>> = chunk
                .iter()
                .map(|file| {
                    let path = Path::new(file);
                    let ancestor = codeowners_engine::ancestor_index_for(&ancestor_cache, path);
                    codeowners.of_index_with_ancestor(path, ancestor)
                })
                .collect();

            // Throttle progress emissions to at most ~10/s. `try_lock` means
            // if another worker already holds the lock we just skip this
            // update — the next chunk will pick it up.
            let handled =
                counter.fetch_add(chunk.len() as u32, Ordering::Relaxed) + chunk.len() as u32;
            if let Ok(mut last) = last_emit.try_lock() {
                let now = Instant::now();
                if now.duration_since(*last) >= Duration::from_millis(100) {
                    *last = now;
                    // Drop the guard before doing IPC so we don't hold it
                    // across a potentially slow emit.
                    drop(last);
                    emit_payload(handled);
                }
            }
            out.into_iter()
        })
        .collect();

    // Always emit a final 100% event so the UI can transition off the
    // progress indicator regardless of throttle timing.
    emit_payload(files_total);

    // -------------------------------------------------------------------
    // Phase 3: single-pass tree assembly using an AHashMap child index.
    //
    // Replaces the previous linear `Vec::position` search per directory
    // descent, turning tree construction from O(N*avg_siblings) into
    // O(N*depth).
    // -------------------------------------------------------------------
    let mut result: DirectoryOwners = DirectoryOwners::new_root();
    for (file_path, owner_idx) in files.iter().zip(owner_indices.iter()) {
        let owner = owner_idx
            .map(|i| owner_strings[i].clone())
            .unwrap_or_default();
        let mut current = &mut result;
        let mut it = file_path.split('/').peekable();
        while let Some(file_path_part) = it.next() {
            let is_last_part = it.peek().is_none();
            if is_last_part {
                current.files.push(FileOwners {
                    name: file_path_part.to_string(),
                    owner: owner.clone(),
                });
            } else {
                let idx = if let Some(&i) = current.child_index.get(file_path_part) {
                    i
                } else {
                    let i = current.directories.len();
                    current
                        .child_index
                        .insert(file_path_part.to_string(), i);
                    current
                        .directories
                        .push(DirectoryOwners::new_named(file_path_part));
                    i
                };
                current = &mut current.directories[idx];
            }
        }
    }
    result
}

struct FileOwners {
    name: String,
    owner: String,
}

impl Serialize for FileOwners {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 2 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("FileOwners", 2)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("owner", &self.owner)?;
        state.end()
    }
}

struct DirectoryOwners {
    /** Directory name. For root folder it is empty */
    name: String,
    directories: Vec<DirectoryOwners>,
    files: Vec<FileOwners>,
    /**
     * @return string which contains all owners in case every files/directories inside have their own owners.
     *   null in other case (for root directory also null)
     */
    owner: Option<String>,
    /// O(1) child-directory lookup by name during tree construction.
    /// Not serialized — it's purely a build-time index and would be dead
    /// weight in the JSON payload. Uses `ahash` (same hasher family as the
    /// ancestor cache) for fast small-string hashing.
    child_index: AHashMap<String, usize>,
}

impl DirectoryOwners {
    fn new_root() -> Self {
        DirectoryOwners {
            name: String::new(),
            directories: Vec::new(),
            files: Vec::new(),
            owner: None,
            child_index: AHashMap::new(),
        }
    }

    fn new_named(name: &str) -> Self {
        DirectoryOwners {
            name: name.to_string(),
            directories: Vec::new(),
            files: Vec::new(),
            owner: None,
            child_index: AHashMap::new(),
        }
    }
}

impl Serialize for DirectoryOwners {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 4 is the number of fields in the struct.
        // `child_index` is intentionally excluded — it's a build-time
        // accelerator, not part of the JSON contract with the frontend.
        let mut state = serializer.serialize_struct("DirectoryOwners", 4)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("directories", &self.directories)?;
        state.serialize_field("files", &self.files)?;
        state.serialize_field("owner", &self.owner)?;
        state.end()
    }
}
