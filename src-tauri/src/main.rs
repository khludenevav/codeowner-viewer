// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{collections::HashMap, process::Command};
pub mod codeowners_file_parser;
use serde::ser::{Serialize, SerializeStruct, Serializer};

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
fn get_all_codeowners_for_branch(abs_repo_path: &str, branch: &str) -> String {
    let all_owners = get_all_codeowners_for_branch_struct(abs_repo_path, branch);
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

fn get_all_codeowners_for_branch_struct(abs_repo_path: &str, branch: &str) -> DirectoryOwners {
    DirectoryOwners {
        name: String::from(""),
        directories: vec![],
        files: vec![],
        owner: Option::None,
    }
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
