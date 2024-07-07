// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_codeowners_content])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_codeowners_content(codeowners_path: &str) -> String {
    format!("Path: {}", codeowners_path);
    let content: String = fs::read_to_string(codeowners_path).unwrap();
    content
}
