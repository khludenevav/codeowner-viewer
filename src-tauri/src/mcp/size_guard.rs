//! Size guard: prune the annotated trie until its DSL body fits under
//! the byte budget, while keeping the header + rules table intact.
//!
//! Strategy: emit the body; if it's over budget, find the deepest
//! subtree whose serialized size contributes the most bytes, mark it as
//! `TRUNCATED` with the sorted set of rule ids it contains, and repeat.
//!
//! The "heaviest subtree" is chosen from **directory nodes only** — you
//! can't truncate a single file — with a minimum descendant-leaf count
//! of 2. Ties break on greater depth (prefer collapsing a small deep
//! branch over a huge shallow one, so the caller can still request the
//! subtree explicitly).

use super::compactor::{
    AnnotatedTree, TrieNode, collect_rule_counts, sort_counts,
};
use super::dsl_emitter::{self, HeaderExtras};
use super::schema::ResponseMode;

pub const DEFAULT_SIZE_LIMIT: usize = 50_000;
/// Give up after this many collapse iterations to guarantee termination
/// even on pathological trees.
const MAX_ITER: usize = 256;

pub struct GuardResult {
    /// Final serialized body (already includes header).
    pub body: String,
    pub truncated: bool,
    /// The untruncated body — only populated when the guard fired, so
    /// the caller can dump it to disk.
    pub full_body: Option<String>,
}

pub fn apply(
    tree: &mut AnnotatedTree,
    mode: ResponseMode,
    limit: usize,
    dump_path_hint: Option<&str>,
) -> GuardResult {
    let extras = HeaderExtras::default();
    let initial = dsl_emitter::emit(tree, mode, &extras);
    if initial.len() <= limit {
        return GuardResult { body: initial, truncated: false, full_body: None };
    }

    // Keep the fully expanded body so callers can dump it to disk before
    // we mutate the tree.
    let full_body = initial;

    let mut iter = 0usize;
    loop {
        iter += 1;
        if iter > MAX_ITER {
            break;
        }
        if !collapse_heaviest(&mut tree.root, 0) {
            break;
        }
        let extras = HeaderExtras {
            truncated: true,
            full_dump_path: dump_path_hint.map(|s| s.to_string()),
        };
        let candidate = dsl_emitter::emit(tree, mode, &extras);
        if candidate.len() <= limit {
            return GuardResult {
                body: candidate,
                truncated: true,
                full_body: Some(full_body),
            };
        }
    }

    // Last resort: even after collapsing everything we could, we're still
    // over budget. Return whatever we have — the header will still be
    // marked truncated and the full body is saved to disk.
    let extras = HeaderExtras {
        truncated: true,
        full_dump_path: dump_path_hint.map(|s| s.to_string()),
    };
    let candidate = dsl_emitter::emit(tree, mode, &extras);
    GuardResult { body: candidate, truncated: true, full_body: Some(full_body) }
}

/// Find the heaviest not-yet-truncated directory subtree and collapse
/// it. Returns `true` if something was collapsed.
///
/// Heaviness = descendant leaf count. Ties broken by greater depth so we
/// collapse specific deep branches first — a huge shallow branch would
/// take the whole response with it.
fn collapse_heaviest(root: &mut TrieNode, root_depth: usize) -> bool {
    // (leaves, depth, path-to-node). We rediscover the path each pass —
    // simpler than juggling &mut aliasing rules.
    let mut best: Option<(usize, usize, Vec<String>)> = None;
    find_candidate(root, root_depth, &mut Vec::new(), &mut best);
    let (_, _, path) = match best {
        Some(x) => x,
        None => return false,
    };
    collapse_at(root, &path);
    true
}

fn find_candidate(
    node: &TrieNode,
    depth: usize,
    path: &mut Vec<String>,
    best: &mut Option<(usize, usize, Vec<String>)>,
) {
    for (key, child) in &node.children {
        if child.truncated.is_none() && child.descendant_leaves >= 2 {
            let candidate = (child.descendant_leaves, depth + 1, {
                let mut p = path.clone();
                p.push(key.clone());
                p
            });
            match best {
                None => *best = Some(candidate),
                Some(b) => {
                    // Larger leaves wins; on tie, greater depth wins.
                    if candidate.0 > b.0
                        || (candidate.0 == b.0 && candidate.1 > b.1)
                    {
                        *best = Some(candidate);
                    }
                }
            }
        }
        path.push(key.clone());
        find_candidate(child, depth + 1, path, best);
        path.pop();
    }
}

fn collapse_at(root: &mut TrieNode, path: &[String]) {
    let mut cursor = root;
    for (i, key) in path.iter().enumerate() {
        let is_last = i + 1 == path.len();
        let next = cursor.children.get_mut(key).expect("path must exist");
        if is_last {
            let counts = collect_rule_counts(next);
            next.truncated = Some(sort_counts(counts));
            next.children.clear();
            next.files.clear();
            return;
        }
        cursor = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codeowners_engine::OwnerResolution;
    use crate::mcp::compactor::build_annotated_tree;

    fn res(owners: &str, line: u32) -> OwnerResolution {
        OwnerResolution {
            owners: owners.to_string(),
            rule_comment: None,
            rule_line_number: line,
        }
    }

    #[test]
    fn small_tree_does_not_trip() {
        let files = vec!["a/one.rs".to_string(), "a/two.rs".to_string()];
        let resolved = vec![res("@x", 12), res("@x", 12)];
        let mut tree = build_annotated_tree(&files, &resolved, None, &[]);
        let r = apply(&mut tree, ResponseMode::Full, DEFAULT_SIZE_LIMIT, None);
        assert!(!r.truncated);
        assert!(r.full_body.is_none());
    }

    #[test]
    fn large_tree_gets_truncated() {
        // Build a synthetic wide+deep tree with two distinct top-level
        // directories so the common-prefix base can't collapse it into
        // a rootless list of files. A 500-byte budget forces the guard
        // to prune at least one subtree.
        let mut files = Vec::new();
        let mut resolved = Vec::new();
        for i in 0..500 {
            files.push(format!("app/big/subdir/file_{i}.rs"));
            // Alternate rule ids so any collapsed subtree carries a
            // mixed `[id:count,...] TRUNCATED` marker instead of the
            // uniform-collapse rendering.
            let rule = if i % 2 == 0 { 12 } else { 13 };
            let owner = if rule == 12 { "@x" } else { "@y" };
            resolved.push(res(owner, rule));
        }
        for i in 0..2 {
            files.push(format!("docs/readme_{i}.md"));
            resolved.push(res("@x", 12));
        }
        let mut tree = build_annotated_tree(&files, &resolved, None, &[]);
        let r = apply(&mut tree, ResponseMode::Full, 500, Some("/tmp/dump.txt"));
        assert!(r.truncated);
        assert!(r.body.contains("TRUNCATED"));
        assert!(r.body.contains("fullDumpPath: /tmp/dump.txt"));
        assert!(r.full_body.is_some());
    }
}
