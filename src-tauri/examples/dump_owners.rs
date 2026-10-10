//! Prints `<path>\t<rule line>\t<owners>` for every file of a repo at a ref,
//! resolved the same way the UI and MCP do. Used by `scripts/compare-codeowners.sh`.
//!
//! cargo run --release --example dump_owners -- <repo> [ref=HEAD]
//!
//! Uses only APIs that exist on older commits too, so the script can copy it
//! into a worktree of any base ref.
use app_lib::{codeowners_engine as engine, codeowners_file_parser as parser};
use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let repo = args.next().expect("usage: dump_owners <repo> [ref]");
    let git_ref = args.next().unwrap_or_else(|| "HEAD".to_string());

    // GitHub's lookup order.
    let rel = [".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"]
        .into_iter()
        .find(|rel| Path::new(&repo).join(rel).is_file())
        .expect("no CODEOWNERS in .github/, root or docs/");
    let text = std::fs::read_to_string(Path::new(&repo).join(rel)).unwrap();
    let owners = parser::from_reader(text.as_bytes());

    let files = engine::get_branch_files_vector(&repo, &git_ref);
    for (file, res) in files.iter().zip(engine::resolve_owners_batch(&owners, &files)) {
        println!("{file}\t{}\t{}", res.rule_line_number, res.owners);
    }
}
