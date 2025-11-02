// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{collections::HashMap, process::Command};
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

    let mut owners_dictionary: HashMap<String, Vec<String>> = HashMap::new();
    for file_path in branch_diff.split("\n") {
        if file_path.is_empty() {
            // it is for latest line
            continue;
        }
        let owner_team = get_joined_codeowners(codeowners.of(file_path));

        owners_dictionary
            .entry(owner_team.unwrap_or(String::new()))
            .and_modify(|e| e.push(file_path.to_string()))
            .or_insert(vec![file_path.to_string()]);
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
}

struct FrontendCodeowner {
    /// Codeowners
    owners: String,
    files: Vec<String>,
}

impl Serialize for FrontendCodeowner {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 2 is the number of fields in the struct.
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
    let mut result: DirectoryOwners = DirectoryOwners {
        name: String::from(""),
        directories: vec![],
        files: vec![],
        owner: Option::None,
    };

    for (file_index, file_path) in files.iter().enumerate() {
        if file_index % 100 == 0 {
            let payload = AllCodeownersProgressPayload {
                files_handled: file_index as u32,
                files_total: files.len() as u32,
                session_id: session_id.to_string(),
            };
            app_handle
                .emit_all("all-codeowners-progress", payload)
                .unwrap();
            println!("handled {file_index} from {}", files.len());
        }
        // if file_index > 2000 {
        //     break; // TODO: we need speedup algorithm. Now it is too long. Uncomment for quick debugging.
        // }
        let owner = get_joined_codeowners(codeowners.of(&file_path)).unwrap_or(String::new());
        let mut current = &mut result;
        let mut it = file_path.split('/').peekable();
        while let Some(file_path_part) = it.next() {
            let is_last_part = it.peek().is_none();
            if is_last_part {
                current.files.push(FileOwners {
                    name: file_path_part.to_string(),
                    owner: String::from(&owner),
                })
            } else {
                if let Some(existing_dir_position) = current
                    .directories
                    .iter()
                    .position(|dir| dir.name == file_path_part)
                {
                    current = &mut current.directories[existing_dir_position];
                } else {
                    let new_dir_owners = DirectoryOwners {
                        name: String::from(file_path_part),
                        directories: vec![],
                        files: vec![],
                        owner: Option::None,
                    };
                    current.directories.push(new_dir_owners);
                    current = current
                        .directories
                        .iter_mut()
                        .find(|dir| dir.name == file_path_part)
                        .expect("Just added this directory, so it should be found")
                }
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
}

impl Serialize for DirectoryOwners {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 4 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("DirectoryOwners", 4)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("directories", &self.directories)?;
        state.serialize_field("files", &self.files)?;
        state.serialize_field("owner", &self.owner)?;
        state.end()
    }
}
