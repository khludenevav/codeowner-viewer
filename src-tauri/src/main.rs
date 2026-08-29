// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    collections::HashMap,
    path::Path,
    process::Command,
    sync::{
        atomic::{AtomicU32, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

use ahash::AHashMap;
use rayon::prelude::*;

pub mod codeowners_file_parser;
use serde::ser::{SerializeStruct, Serializer};
use serde::Serialize;
use tauri::Manager;

extern crate pretty_assertions;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_branch_files,
            get_all_codeowners_for_branch,
            get_changed_codeowners_for_branch,
            get_codeowners_for_branch_file,
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
        // rule's inline `#!` comment. Halves the matching work compared
        // to calling `of()` + `comment_of()` back-to-back.
        let (owners_opt, comment_opt) = codeowners.of_with_comment(file_path);
        let owner_team = get_joined_codeowners(owners_opt);
        let comment = comment_opt.map(|s| s.to_string());
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
    let output = Command::new("git")
        .current_dir(abs_repo_path)
        .arg("--no-pager")
        .arg("diff")
        .arg("--name-only")
        .arg(format!("origin/main...{branch}"))
        .output()
        .expect("git command failed");

    if !output.status.success() {
        println!("Error: {}", String::from_utf8_lossy(&output.stderr));
    }
    let content: String = String::from_utf8_lossy(&output.stdout).to_string();
    content
}

/** Return all files in the repo for passed branch */
fn get_branch_files_vector(abs_repo_path: &str, branch: &str) -> Vec<String> {
    let output = Command::new("git")
        .current_dir(abs_repo_path)
        .arg("ls-tree")
        .arg("-r")
        .arg(branch)
        .arg("--name-only")
        .output()
        .expect("git command failed");

    if !output.status.success() {
        println!("Error: {}", String::from_utf8_lossy(&output.stderr));
    }

    let mut branch_files: Vec<String> = Vec::new();
    for file_path in String::from_utf8_lossy(&output.stdout)
        .to_string()
        .split("\n")
    {
        // it is for latest line
        if !file_path.is_empty() {
            branch_files.push(file_path.to_string());
        }
    }
    branch_files
}

/** Returns comments for codeowners file of passed branch */
fn get_codeowners_content(abs_repo_path: &str, branch: &str) -> String {
    let output = Command::new("git")
        .current_dir(abs_repo_path)
        .arg("--no-pager")
        .arg("show")
        .arg(format!("{branch}:CODEOWNERS"))
        .output()
        .expect("git command failed");
    if !output.status.success() {
        println!("Error: {}", String::from_utf8_lossy(&output.stderr));
    }
    let content: String = String::from_utf8_lossy(&output.stdout).to_string();
    content
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
            .emit_all("all-codeowners-progress", payload)
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
    let ancestor_cache = build_ancestor_cache(&codeowners, &files);

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
                    let ancestor = ancestor_index_for(&ancestor_cache, path);
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

/// Precompute `ancestor_first_i(dir)` for every unique parent directory
/// appearing in `files`. The result maps the canonical directory string
/// (as produced by `Path::to_str()`) to the first rule index that matches
/// the directory or one of its ancestors, ignoring rules ending with `/*`.
///
/// Runs in three passes:
///   1. Collect every unique directory (including all ancestors) — cheap.
///   2. Compute `direct_match_index_at` for each unique directory in
///      parallel across rayon workers — this is the pattern-matching heavy
///      pass, and the reason we parallelize it.
///   3. Serially combine each directory's direct match with its parent's
///      cached ancestor result. This is bounded by directory depth and
///      pure integer work, so a single-threaded sweep is fine.
fn build_ancestor_cache(
    codeowners: &codeowners_file_parser::Owners,
    files: &[String],
) -> AHashMap<String, Option<usize>> {
    // Pass 1: gather unique directories, including all ancestors.
    let mut unique_dirs: AHashMap<String, ()> = AHashMap::new();
    for file in files {
        let path = Path::new(file);
        let mut cur = path.parent();
        while let Some(dir) = cur {
            let key = dir.to_str().unwrap_or("");
            // If this dir was already inserted we can stop — every
            // ancestor above it is also already in the set.
            if unique_dirs.insert(key.to_string(), ()).is_some() {
                break;
            }
            cur = dir.parent();
        }
    }
    let dirs: Vec<String> = unique_dirs.into_keys().collect();

    // Pass 2: parallel direct-match probe. This is the expensive part —
    // each `direct_match_index_at` call scans up to P patterns. Doing it
    // in parallel across cores yields the biggest win for repos with
    // thousands of unique directories.
    let direct: Vec<(String, Option<usize>)> = dirs
        .par_iter()
        .map(|d| {
            let idx = codeowners.direct_match_index_at(Path::new(d));
            (d.clone(), idx)
        })
        .collect();
    let mut direct_map: AHashMap<String, Option<usize>> =
        AHashMap::with_capacity(direct.len());
    for (d, i) in direct {
        direct_map.insert(d, i);
    }

    // Pass 3: serial ancestor combine. Each directory's ancestor result is
    // `min(direct(self), ancestor(parent))`. Because we already have every
    // ancestor in `direct_map`, the recursive helper is guaranteed to
    // terminate quickly and each memoized entry is computed exactly once.
    let mut cache: AHashMap<String, Option<usize>> =
        AHashMap::with_capacity(direct_map.len());
    // Snapshot the keys so we can iterate while also mutating `cache`.
    let keys: Vec<String> = direct_map.keys().cloned().collect();
    for k in keys {
        combine_ancestor_index(&direct_map, &mut cache, &k);
    }
    cache
}

/// Recursive combine step for [`build_ancestor_cache`]. Populates `cache`
/// for `dir_key` by merging its own direct match (looked up in `direct`)
/// with its parent's already-cached ancestor result.
fn combine_ancestor_index(
    direct: &AHashMap<String, Option<usize>>,
    cache: &mut AHashMap<String, Option<usize>>,
    dir_key: &str,
) -> Option<usize> {
    if let Some(v) = cache.get(dir_key) {
        return *v;
    }
    let self_match = direct.get(dir_key).copied().flatten();
    let parent_match = match Path::new(dir_key).parent() {
        Some(p) => {
            let pk = p.to_str().unwrap_or("");
            if pk.is_empty() && dir_key.is_empty() {
                None
            } else if direct.contains_key(pk) {
                combine_ancestor_index(direct, cache, pk)
            } else {
                None
            }
        }
        None => None,
    };
    let result = match (self_match, parent_match) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    cache.insert(dir_key.to_string(), result);
    result
}

/// Read the ancestor-first-index for a file's parent directory out of the
/// cache. Files with no parent (bare filenames) fall back to `None`, which
/// makes `Owners::of_index_with_ancestor` fall through to a full pattern
/// scan — equivalent to the original algorithm's behavior for such paths.
fn ancestor_index_for(
    cache: &AHashMap<String, Option<usize>>,
    file_path: &Path,
) -> Option<usize> {
    let parent = file_path.parent()?;
    let key = parent.to_str().unwrap_or("");
    cache.get(key).copied().flatten()
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
