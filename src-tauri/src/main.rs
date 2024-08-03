// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::time::Instant;
use std::{collections::HashMap, process::Command};

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_codeowners_content,
            get_branch_diff,
            get_changed_codeowners_for_branch
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug)]
// Later we need all of these
#[allow(dead_code)]
struct CodeownerLine {
    line: usize,
    pattern: String,
    owners: Vec<String>,
    required: bool,
    gitignore: Gitignore,
}

/** Branch supports passing HEAD */
#[tauri::command]
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

/** Returns difference between passed branch and main */
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

fn parse_codeowners_file(codeowners_file_content: String) -> Vec<CodeownerLine> {
    let mut lines: Vec<CodeownerLine> = codeowners_file_content
        .split("\n")
        .enumerate()
        .filter_map(|(line_index, str_line)| -> Option<CodeownerLine> {
            if str_line.is_empty() {
                return None;
            }
            let splitted_line = str_line
                .split('#')
                .map(|x| x.trim())
                .take(2)
                .collect::<Vec<&str>>();

            if let Some(declarations) = splitted_line.first() {
                let declarations = declarations.trim();
                if declarations.is_empty() {
                    return None;
                }
                let splitted_declaration = declarations
                    .split_whitespace()
                    .map(|x| x.trim())
                    .collect::<Vec<&str>>();
                let pattern = if let Some(pattern) = splitted_declaration.first() {
                    if pattern.len() <= 0 {
                        return None;
                    }
                    pattern.to_string()
                } else {
                    return None;
                };

                let owners: Vec<String> = splitted_declaration
                    .iter()
                    .skip(1)
                    .map(|v| v.to_string())
                    .collect();
                let required = if let Some(comments) = splitted_line.get(1) {
                    comments.contains("!required")
                } else {
                    false
                };

                let mut builder = GitignoreBuilder::new("");
                builder.add_line(None, &pattern).unwrap();
                let gitignore = builder.build().unwrap();

                return Some(CodeownerLine {
                    line: line_index,
                    pattern,
                    owners,
                    required,
                    gitignore,
                });
            }
            None
        })
        .collect();
    lines.reverse();
    lines
}

fn get_owner_team(parsed_codeowners: &Vec<CodeownerLine>, file_path: &str) -> Option<String> {
    for entry in parsed_codeowners {
        if entry
            .gitignore
            .matched_path_or_any_parents(file_path, false)
            .is_ignore()
        {
            return Some(entry.owners.join(", ").to_string());
        }
    }
    None
}

/** Key is team or empty, value is changed files for branch */
#[tauri::command(async)]
fn get_changed_codeowners_for_branch(abs_repo_path: &str, branch: &str) -> String {
    let codeowners_content = get_codeowners_content(abs_repo_path, branch);
    let parsed_codeowners = parse_codeowners_file(codeowners_content);
    let branch_diff = get_branch_diff(abs_repo_path, branch);

    let now = Instant::now();
    let mut owners_dictionary: HashMap<String, Vec<String>> = HashMap::new();
    for file_path in branch_diff.split("\n") {
        if file_path.is_empty() {
            // it is for latest line
            continue;
        }
        let owner_team = get_owner_team(&parsed_codeowners, file_path);

        owners_dictionary
            .entry(owner_team.unwrap_or(String::new()))
            .and_modify(|e| e.push(file_path.to_string()))
            .or_insert(vec![file_path.to_string()]);
    }
    let elapsed = now.elapsed();
    println!("Codeowners calculation: {:.2?}", elapsed);

    serde_json::to_string(&owners_dictionary).unwrap()
}

#[test]
fn test_gitignore_builder() {
    let mut builder = GitignoreBuilder::new("");
    builder.add_line(None, "/client/apps/dashboard").unwrap();
    let gitignore = builder.build().unwrap();

    let check_is_ignore = |s: &str| gitignore.matched_path_or_any_parents(s, false).is_ignore();
    assert!(check_is_ignore(
        "client/apps/dashboard/src/components/Sidebar/SidebarLayout/SidebarLayout.module.scss"
    ));
    assert!(check_is_ignore("client/apps/dashboard/any_file.ts"));
    assert!(!check_is_ignore("client/apps/any_file.ts"));
}

#[test]
fn test_parse_codeowners_file() {
    let parsed = parse_codeowners_file(String::from(
        "   /client/apps/dashboard owner1\n/client/apps/docs owner2 #!required",
    ));

    assert_eq!(parsed.len(), 2);

    let first = parsed.get(1).unwrap();
    assert_eq!(first.owners, vec![String::from("owner1")]);
    assert_eq!(first.pattern, String::from("/client/apps/dashboard"));
    assert_eq!(first.required, false);

    let second = parsed.get(0).unwrap();
    assert_eq!(second.owners, vec![String::from("owner2")]);
    assert_eq!(second.pattern, String::from("/client/apps/docs"));
    assert_eq!(second.required, true);
}
