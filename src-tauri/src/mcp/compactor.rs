//! Trie-based collapser for the `compact` MCP response mode.
//!
//! Given a full flat map `path → (owners, ruleComment)` plus the set of
//! reachable files under the repo root, produce a minimal array grouped
//! by owner-set that collapses full-coverage directories.

use std::collections::BTreeMap;

use crate::codeowners_engine::OwnerResolution;

use super::schema::CompactEntry;

/// Collapse `resolved` (in the same order as `expanded_files`) into the
/// smallest possible compact response.
///
/// `reachable_files` is the full universe (branch tree or changed set)
/// used to decide "does every file in this directory appear in the
/// request's expanded set?" — a prerequisite for collapsing.
pub fn compact_response(
    expanded_files: &[String],
    resolved: &[OwnerResolution],
    reachable_files: &[String],
) -> Vec<CompactEntry> {
    debug_assert_eq!(expanded_files.len(), resolved.len());

    let reachable_count = count_by_dir(reachable_files);
    let expanded_count = count_by_dir(expanded_files);

    let mut root = TrieNode::new();
    for (path, res) in expanded_files.iter().zip(resolved.iter()) {
        let parts: Vec<&str> = path.split('/').collect();
        insert(
            &mut root,
            &parts,
            0,
            (res.owners.clone(), res.rule_comment.clone()),
        );
    }

    compute(&mut root, "", &reachable_count, &expanded_count);

    let mut groups: BTreeMap<(String, Option<String>), Vec<String>> =
        BTreeMap::new();
    walk(&root, "", &mut groups);

    let mut out: Vec<CompactEntry> = groups
        .into_iter()
        .map(|((owners, comment), paths)| CompactEntry {
            owners,
            paths,
            rule_comment: comment,
        })
        .collect();
    out.sort_by(|a, b| a.owners.cmp(&b.owners).then_with(|| a.paths.cmp(&b.paths)));
    out
}

fn count_by_dir(files: &[String]) -> BTreeMap<String, usize> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for f in files {
        let mut cursor = String::new();
        counts.entry(cursor.clone()).and_modify(|c| *c += 1).or_insert(1);
        let parts: Vec<&str> = f.split('/').collect();
        for i in 0..parts.len().saturating_sub(1) {
            if !cursor.is_empty() {
                cursor.push('/');
            }
            cursor.push_str(parts[i]);
            counts.entry(cursor.clone()).and_modify(|c| *c += 1).or_insert(1);
        }
    }
    counts
}

fn insert(
    node: &mut TrieNode,
    parts: &[&str],
    depth: usize,
    val: (String, Option<String>),
) {
    if depth == parts.len().saturating_sub(1) {
        node.own_files.insert(parts[depth].to_string(), val);
        return;
    }
    let name = parts[depth].to_string();
    let child = node.children.entry(name).or_insert_with(TrieNode::new);
    insert(child, parts, depth + 1, val);
}

fn compute(
    node: &mut TrieNode,
    key: &str,
    reachable_count: &BTreeMap<String, usize>,
    expanded_count: &BTreeMap<String, usize>,
) {
    let child_keys: Vec<String> = node.children.keys().cloned().collect();
    for name in child_keys {
        let child_key = if key.is_empty() {
            name.clone()
        } else {
            format!("{key}/{name}")
        };
        if let Some(child) = node.children.get_mut(&name) {
            compute(child, &child_key, reachable_count, expanded_count);
        }
    }

    let need = reachable_count.get(key).copied().unwrap_or(0);
    let have = expanded_count.get(key).copied().unwrap_or(0);
    node.fully_covered = need > 0 && have == need;

    if !node.fully_covered {
        node.collapsed = None;
        return;
    }

    let mut common: Option<(String, Option<String>)> = None;
    let mut ok = true;

    for val in node.own_files.values() {
        match &common {
            None => common = Some(val.clone()),
            Some(c) if c == val => {}
            Some(_) => {
                ok = false;
                break;
            }
        }
    }
    if ok {
        for child in node.children.values() {
            if let Some(c) = &child.collapsed {
                match &common {
                    None => common = Some(c.clone()),
                    Some(cur) if cur == c => {}
                    Some(_) => {
                        ok = false;
                        break;
                    }
                }
            } else {
                ok = false;
                break;
            }
        }
    }

    node.collapsed = if ok { common } else { None };
}

fn walk(
    node: &TrieNode,
    key: &str,
    groups: &mut BTreeMap<(String, Option<String>), Vec<String>>,
) {
    if let Some(c) = &node.collapsed {
        let entry = if key.is_empty() {
            ".".to_string()
        } else {
            key.to_string()
        };
        groups.entry(c.clone()).or_default().push(entry);
        return;
    }
    for (name, val) in &node.own_files {
        let leaf_key = if key.is_empty() {
            name.clone()
        } else {
            format!("{key}/{name}")
        };
        groups.entry(val.clone()).or_default().push(leaf_key);
    }
    for (name, child) in &node.children {
        let child_key = if key.is_empty() {
            name.clone()
        } else {
            format!("{key}/{name}")
        };
        walk(child, &child_key, groups);
    }
}

struct TrieNode {
    children: BTreeMap<String, TrieNode>,
    own_files: BTreeMap<String, (String, Option<String>)>,
    collapsed: Option<(String, Option<String>)>,
    fully_covered: bool,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: BTreeMap::new(),
            own_files: BTreeMap::new(),
            collapsed: None,
            fully_covered: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codeowners_engine::OwnerResolution;

    fn res(owners: &str, comment: Option<&str>) -> OwnerResolution {
        OwnerResolution {
            owners: owners.to_string(),
            rule_comment: comment.map(|s| s.to_string()),
            rule_line_number: 0,
        }
    }

    #[test]
    fn collapses_full_coverage_uniform_owners() {
        let reachable = vec![
            "a/b/c.txt".to_string(),
            "a/b/d.txt".to_string(),
        ];
        let expanded = reachable.clone();
        let resolved = vec![res("@team", None), res("@team", None)];
        let out = compact_response(&expanded, &resolved, &reachable);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].owners, "@team");
        assert_eq!(out[0].paths, vec![".".to_string()]);
    }

    #[test]
    fn keeps_files_when_owners_differ() {
        let reachable = vec!["a/b/c.txt".to_string(), "a/b/d.txt".to_string()];
        let expanded = reachable.clone();
        let resolved = vec![res("@a", None), res("@b", None)];
        let out = compact_response(&expanded, &resolved, &reachable);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn does_not_collapse_partial_coverage() {
        let reachable = vec![
            "a/b/c.txt".to_string(),
            "a/b/d.txt".to_string(),
        ];
        let expanded = vec!["a/b/c.txt".to_string()];
        let resolved = vec![res("@team", None)];
        let out = compact_response(&expanded, &resolved, &reachable);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].paths, vec!["a/b/c.txt".to_string()]);
    }

    #[test]
    fn collapses_all_unowned_subtree() {
        let reachable = vec!["x/y.txt".to_string(), "x/z.txt".to_string()];
        let expanded = reachable.clone();
        let resolved = vec![res("", None), res("", None)];
        let out = compact_response(&expanded, &resolved, &reachable);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].owners, "");
    }

    #[test]
    fn distinct_comments_prevent_collapse() {
        let reachable = vec!["a/b.txt".to_string(), "a/c.txt".to_string()];
        let expanded = reachable.clone();
        let resolved = vec![res("@team", Some("#required")), res("@team", None)];
        let out = compact_response(&expanded, &resolved, &reachable);
        assert_eq!(out.len(), 2);
    }
}
