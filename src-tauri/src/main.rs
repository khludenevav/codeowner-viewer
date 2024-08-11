// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{collections::HashMap, process::Command};
pub mod codeowners;
use serde::ser::{Serialize, SerializeStruct, Serializer};

#[macro_use]
extern crate pretty_assertions;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_branch_diff,
            get_changed_codeowners_for_branch
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/** Returns difference as list of changed files between passed branch and main */
#[tauri::command]
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

/** Key is team or empty, value is changed files for branch */
#[tauri::command(async)]
fn get_changed_codeowners_for_branch(abs_repo_path: &str, branch: &str) -> String {
    let codeowners_content = get_codeowners_content(abs_repo_path, branch);
    let codeowners = codeowners::from_reader(codeowners_content.as_bytes());
    let branch_diff = get_branch_diff(abs_repo_path, branch);

    let mut owners_dictionary: HashMap<String, Vec<String>> = HashMap::new();
    for file_path in branch_diff.split("\n") {
        if file_path.is_empty() {
            // it is for latest line
            continue;
        }

        let owner_team = match codeowners.of(file_path) {
            None => None,
            Some(owners) => Some(
                owners
                    .iter()
                    .map(|owner| format!("{owner}"))
                    .collect::<Vec<String>>()
                    .join(", "),
            ),
        };

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
