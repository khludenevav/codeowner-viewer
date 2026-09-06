//! Build an annotated trie from resolved codeowners data.
//!
//! The trie is the intermediate structure consumed by `dsl_emitter` (to
//! produce the final DSL body) and by `size_guard` (to prune heavy
//! subtrees when the body would overflow the 50 KB budget).
//!
//! Each `TrieNode` carries its subtree's **prevailing rule id** (majority
//! among descendant leaves, ties broken by the smaller rule id). This is
//! what the DSL turns into a `dir/ N` line when it differs from the
//! enclosing default.
//!
//! Single-child directory chains are collapsed here so the emitter can
//! output `a/b/c/d/` instead of four nested lines.
//!
//! Rule id `0` is the sentinel for "unowned" (no CODEOWNERS rule
//! matched). It is a valid id both in the rules table and in the tree —
//! CODEOWNERS files are 1-indexed so `0` never collides.

use std::collections::BTreeMap;

use crate::codeowners_engine::OwnerResolution;

pub const UNOWNED_RULE_ID: u32 = 0;

#[derive(Debug, Clone)]
pub struct RuleInfo {
    /// Space-separated owner handles. Empty string means unowned.
    pub owners: String,
    /// Comment attached to the rule (leading `#` and whitespace stripped).
    /// `None` if the rule had no comment.
    pub comment: Option<String>,
}

/// Everything the emitter needs to serialize a response.
#[derive(Debug, Clone)]
pub struct AnnotatedTree {
    /// Directory prefix common to every emitted path; stripped from the
    /// tree keys and echoed in the header. Empty string if the emitted
    /// paths do not share a common directory (or if the tree is empty).
    pub base: String,
    /// Rule id that applies to anything not otherwise listed. `None`
    /// when the tree is empty.
    pub root_default: Option<u32>,
    /// Rule table, keyed by CODEOWNERS line number. `0` is the special
    /// "unowned" entry when at least one such file appears in the tree.
    pub rules: BTreeMap<u32, RuleInfo>,
    /// Root trie node. `root.key` is empty; its children are the
    /// top-level directories relative to `base`.
    pub root: TrieNode,
}

#[derive(Debug, Clone)]
pub struct TrieNode {
    /// Path segment for this node. May contain `/` if a single-child
    /// chain was collapsed (e.g. `com/fivetran/app`).
    pub key: String,
    /// Child directories keyed by their (possibly-collapsed) segment.
    pub children: BTreeMap<String, TrieNode>,
    /// Files that live directly in this node, keyed by filename.
    /// Value is the rule id (0 = unowned).
    pub files: BTreeMap<String, u32>,
    /// Majority rule id among descendant leaves of this subtree. Ties
    /// broken by the smaller rule id. `0` means "no leaves or all
    /// unowned"; the emitter treats it the same as any other id.
    pub prevailing: u32,
    /// If `Some`, this subtree has been collapsed for size and the
    /// carried vec is the ordered `(rule_id, count)` list of rules
    /// that would otherwise appear inside — sorted by descending
    /// count with rule id as the tiebreaker. Children/files are
    /// cleared when this is set.
    pub truncated: Option<Vec<(u32, usize)>>,
    /// Descendant leaf count (files anywhere under this node). Used by
    /// the size guard to pick the heaviest subtree to collapse.
    pub descendant_leaves: usize,
}

impl TrieNode {
    fn new(key: String) -> Self {
        Self {
            key,
            children: BTreeMap::new(),
            files: BTreeMap::new(),
            prevailing: UNOWNED_RULE_ID,
            truncated: None,
            descendant_leaves: 0,
        }
    }
}

/// Build the annotated tree from a batch of resolved files.
///
/// `expanded_files` and `resolved` must be in the same order and have
/// the same length.
///
/// If `max_depth` is set and `user_paths` is non-empty, each user path
/// that resolves to a directory in the raw trie is used as an anchor:
/// subtrees deeper than `max_depth` levels below that anchor and that
/// contain more than one distinct rule id are collapsed to a
/// `TRUNCATED` marker before the tree is chain-collapsed.
pub fn build_annotated_tree(
    expanded_files: &[String],
    resolved: &[OwnerResolution],
    max_depth: Option<u32>,
    user_paths: &[String],
) -> AnnotatedTree {
    debug_assert_eq!(expanded_files.len(), resolved.len());

    // --- Rules table (dedup by line number, 0 == unowned).
    let mut rules: BTreeMap<u32, RuleInfo> = BTreeMap::new();
    for res in resolved {
        let id = res.rule_line_number;
        rules.entry(id).or_insert_with(|| RuleInfo {
            owners: res.owners.clone(),
            comment: normalize_comment(res.rule_comment.as_deref()),
        });
    }

    // --- Compute `base` = longest common directory prefix.
    let base = longest_common_dir_prefix(expanded_files);

    // --- Build the raw trie on base-relative paths.
    let mut root = TrieNode::new(String::new());
    for (path, res) in expanded_files.iter().zip(resolved.iter()) {
        let relative = strip_base(path, &base);
        insert_file(&mut root, &relative, res.rule_line_number);
    }

    // --- Apply per-path max-depth truncation on the raw (uncollapsed)
    //     trie so depth reflects real directory levels, not any
    //     later chain merges.
    if let Some(md) = max_depth {
        if !user_paths.is_empty() {
            apply_max_depth(&mut root, &base, user_paths, md);
        }
    }

    // --- Collapse single-child directory chains.
    collapse_chains(&mut root);

    // --- Fill in `prevailing` + `descendant_leaves` bottom-up.
    let leaf_count = annotate(&mut root);
    let root_default = if leaf_count == 0 { None } else { Some(root.prevailing) };

    AnnotatedTree { base, root_default, rules, root }
}

/// Walk to each user-path anchor in the raw trie and truncate any
/// non-uniform subtree deeper than `max_depth` levels beneath it.
fn apply_max_depth(root: &mut TrieNode, base: &str, user_paths: &[String], max_depth: u32) {
    for path in user_paths {
        if path_is_glob(path) {
            continue;
        }
        let relative = strip_base(path, base);
        let segments: Vec<&str> =
            relative.split('/').filter(|s| !s.is_empty()).collect();
        truncate_below_from(root, &segments, max_depth);
    }
}

fn path_is_glob(path: &str) -> bool {
    path.chars().any(|c| matches!(c, '*' | '?' | '[' | '{' | '!'))
}

/// Descend into `segments`, then truncate any subtree deeper than
/// `max_depth` levels below the anchor. Silently no-ops if the path
/// does not resolve to a directory.
///
/// Depth accounting matches the user model: the anchor line lives at
/// depth 0. Its immediate children (files or subdirs) are at depth 1.
/// A subdir at depth `d` holds its own files/children at depth `d+1`.
///
/// Special case: when the anchor resolves to the root (empty segments),
/// the root itself is invisible in the emitter, so we apply the depth
/// budget to each root child independently, treating every top-level
/// directory as its own anchor.
fn truncate_below_from(root: &mut TrieNode, segments: &[&str], max_depth: u32) {
    if segments.is_empty() {
        for child in root.children.values_mut() {
            truncate_recursive(child, 0, max_depth);
        }
        return;
    }
    let mut cursor: &mut TrieNode = root;
    for seg in segments {
        match cursor.children.get_mut(*seg) {
            Some(child) => cursor = child,
            None => return,
        }
    }
    truncate_recursive(cursor, 0, max_depth);
}

fn truncate_recursive(node: &mut TrieNode, node_line_depth: u32, max_depth: u32) {
    let child_line_depth = node_line_depth + 1;
    if child_line_depth > max_depth {
        // Everything inside this node is past the budget: its own files
        // and any child directories all sit at `child_line_depth`.
        collapse_past_budget(node);
        return;
    }
    for child in node.children.values_mut() {
        truncate_recursive(child, child_line_depth, max_depth);
    }
}

/// Collapse a subtree that is past the depth budget:
///   * empty → do nothing;
///   * otherwise → record the rule/count breakdown in `truncated` and
///     clear files/children. The emitter renders single-rule truncated
///     subtrees as a plain `dir/ <rule>` line (they are indistinguishable
///     from a natural uniform subtree), and multi-rule ones as a
///     `[id:count,...] TRUNCATED` marker.
fn collapse_past_budget(node: &mut TrieNode) {
    if node.truncated.is_some() {
        return;
    }
    let counts = collect_rule_counts(node);
    if counts.is_empty() {
        return;
    }
    node.truncated = Some(sort_counts(counts));
    node.children.clear();
    node.files.clear();
}

/// Recursively count how many leaf files each rule id owns inside a
/// subtree. Files inside an already-truncated node are represented by
/// that node's carried stats.
pub fn collect_rule_counts(node: &TrieNode) -> BTreeMap<u32, usize> {
    let mut out: BTreeMap<u32, usize> = BTreeMap::new();
    collect_rule_counts_into(node, &mut out);
    out
}

fn collect_rule_counts_into(node: &TrieNode, out: &mut BTreeMap<u32, usize>) {
    for id in node.files.values() {
        *out.entry(*id).or_insert(0) += 1;
    }
    if let Some(stats) = &node.truncated {
        for (id, count) in stats {
            *out.entry(*id).or_insert(0) += *count;
        }
    }
    for child in node.children.values() {
        collect_rule_counts_into(child, out);
    }
}

/// Order a rule-count map for emission: highest count first, ties
/// broken by ascending rule id (stable, easy to skim).
pub fn sort_counts(counts: BTreeMap<u32, usize>) -> Vec<(u32, usize)> {
    let mut v: Vec<(u32, usize)> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

fn insert_file(node: &mut TrieNode, relative: &str, rule_id: u32) {
    if relative.is_empty() {
        return;
    }
    let mut parts: Vec<&str> = relative.split('/').collect();
    let file = parts.pop().unwrap_or("");
    let mut cursor = node;
    for part in parts {
        cursor = cursor
            .children
            .entry(part.to_string())
            .or_insert_with(|| TrieNode::new(part.to_string()));
    }
    cursor.files.insert(file.to_string(), rule_id);
}

fn collapse_chains(node: &mut TrieNode) {
    for child in node.children.values_mut() {
        collapse_chains(child);
    }
    // A node collapses upward into its parent iff:
    //   - it has exactly one child directory,
    //   - it has no files of its own.
    // We rebuild `children` after inspecting to avoid mutating during
    // iteration.
    let child_keys: Vec<String> = node.children.keys().cloned().collect();
    let mut rebuilt: BTreeMap<String, TrieNode> = BTreeMap::new();
    for key in child_keys {
        let mut child = node.children.remove(&key).unwrap();
        while child.files.is_empty() && child.children.len() == 1 {
            let (gc_key, gc_node) = child.children.iter().next().unwrap();
            let merged_key = format!("{}/{}", child.key, gc_key);
            let mut gc = gc_node.clone();
            gc.key = merged_key;
            child = gc;
        }
        rebuilt.insert(child.key.clone(), child);
    }
    node.children = rebuilt;
}

fn annotate(node: &mut TrieNode) -> usize {
    // A truncated subtree carries pre-computed rule/count stats. Trust
    // them and short-circuit — otherwise we'd recount over the cleared
    // `.files` / `.children` and lose the leaf accounting.
    if let Some(stats) = node.truncated.as_ref() {
        let total: usize = stats.iter().map(|(_, c)| *c).sum();
        node.descendant_leaves = total;
        node.prevailing = stats
            .iter()
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
            .map(|(id, _)| *id)
            .unwrap_or(UNOWNED_RULE_ID);
        return total;
    }

    let mut counts: BTreeMap<u32, usize> = BTreeMap::new();
    let mut total = 0usize;

    for rule in node.files.values() {
        *counts.entry(*rule).or_insert(0) += 1;
        total += 1;
    }
    for child in node.children.values_mut() {
        let leaves = annotate(child);
        total += leaves;
        // Weight the child by its descendant leaves. The child's
        // `prevailing` already reflects its subtree's majority.
        *counts.entry(child.prevailing).or_insert(0) += leaves;
    }

    node.descendant_leaves = total;
    node.prevailing = if total == 0 {
        UNOWNED_RULE_ID
    } else {
        // Max by count, tie-break by smaller rule id.
        counts
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
            .map(|(id, _)| *id)
            .unwrap_or(UNOWNED_RULE_ID)
    };
    total
}

/// Strip a leading `#` and surrounding whitespace from a raw comment.
/// Returns `None` if the input is `None` or empty after stripping.
fn normalize_comment(raw: Option<&str>) -> Option<String> {
    let text = raw?.trim();
    let stripped = text.strip_prefix('#').unwrap_or(text).trim();
    if stripped.is_empty() {
        None
    } else {
        Some(stripped.to_string())
    }
}

fn longest_common_dir_prefix(paths: &[String]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let first_dirs: Vec<&str> = dir_parts(&paths[0]);
    let mut common = first_dirs.len();
    for p in &paths[1..] {
        let parts = dir_parts(p);
        let mut i = 0;
        while i < common && i < parts.len() && parts[i] == first_dirs[i] {
            i += 1;
        }
        common = i;
        if common == 0 {
            break;
        }
    }
    first_dirs[..common].join("/")
}

/// Return the directory components of a path (excluding the filename).
fn dir_parts(path: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = path.split('/').collect();
    if !parts.is_empty() {
        parts.pop();
    }
    parts
}

fn strip_base(path: &str, base: &str) -> String {
    if base.is_empty() {
        return path.to_string();
    }
    if path == base {
        return String::new();
    }
    let prefix = format!("{base}/");
    path.strip_prefix(&prefix).unwrap_or(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res(owners: &str, comment: Option<&str>, line: u32) -> OwnerResolution {
        OwnerResolution {
            owners: owners.to_string(),
            rule_comment: comment.map(|s| s.to_string()),
            rule_line_number: line,
        }
    }

    #[test]
    fn root_default_is_majority_rule() {
        let files = vec![
            "a/one.rs".to_string(),
            "a/two.rs".to_string(),
            "a/three.rs".to_string(),
            "a/four.rs".to_string(),
        ];
        let resolved = vec![
            res("@x", None, 12),
            res("@x", None, 12),
            res("@x", None, 12),
            res("@y", None, 47),
        ];
        let tree = build_annotated_tree(&files, &resolved, None, &[]);
        assert_eq!(tree.root_default, Some(12));
        assert_eq!(tree.base, "a");
    }

    #[test]
    fn single_child_chain_collapses() {
        let files = vec!["a/b/c/d/file.rs".to_string()];
        let resolved = vec![res("@x", None, 12)];
        let tree = build_annotated_tree(&files, &resolved, None, &[]);
        // base is "a/b/c/d" so tree has just "file.rs" at root.
        assert_eq!(tree.base, "a/b/c/d");
        assert!(tree.root.children.is_empty());
        assert_eq!(tree.root.files.get("file.rs"), Some(&12));
    }

    #[test]
    fn chain_collapses_within_tree() {
        let files = vec![
            "app/x/leaf.rs".to_string(),
            "app/y/deep/nested/thing.rs".to_string(),
        ];
        let resolved = vec![res("@x", None, 12), res("@y", None, 47)];
        let tree = build_annotated_tree(&files, &resolved, None, &[]);
        assert_eq!(tree.base, "app");
        // "y/deep/nested" is a single-child chain and should be collapsed.
        assert!(tree.root.children.contains_key("y/deep/nested"));
        assert!(tree.root.children.contains_key("x"));
    }

    #[test]
    fn rules_table_dedups_by_line() {
        let files = vec![
            "a/one.rs".to_string(),
            "a/two.rs".to_string(),
            "a/three.rs".to_string(),
        ];
        let resolved = vec![
            res("@x", Some("#!required"), 47),
            res("@x", Some("#!required"), 47),
            res("@y", None, 55),
        ];
        let tree = build_annotated_tree(&files, &resolved, None, &[]);
        assert_eq!(tree.rules.len(), 2);
        assert_eq!(tree.rules[&47].owners, "@x");
        assert_eq!(tree.rules[&47].comment.as_deref(), Some("!required"));
        assert_eq!(tree.rules[&55].comment, None);
    }

    #[test]
    fn unowned_gets_rule_zero() {
        let files = vec!["a/x.rs".to_string()];
        let resolved = vec![res("", None, 0)];
        let tree = build_annotated_tree(&files, &resolved, None, &[]);
        assert!(tree.rules.contains_key(&0));
        assert_eq!(tree.rules[&0].owners, "");
    }

    #[test]
    fn no_common_prefix_leaves_base_empty() {
        let files = vec!["a/x.rs".to_string(), "b/y.rs".to_string()];
        let resolved = vec![res("@x", None, 12), res("@y", None, 47)];
        let tree = build_annotated_tree(&files, &resolved, None, &[]);
        assert_eq!(tree.base, "");
    }

    #[test]
    fn max_depth_truncates_non_uniform_frontier() {
        // Non-common-dir top-level "other" keeps base="" so the anchor
        // resolves to the concrete "backstage" node. api/ mixes rules
        // 10+20 → will render as `api/ [10:1,20:1] TRUNCATED`. db/ is
        // uniform → collapses silently to `db/ 30`.
        let files = vec![
            "backstage/api/one.rs".to_string(),
            "backstage/api/two.rs".to_string(),
            "backstage/db/three.rs".to_string(),
            "other/z.rs".to_string(),
        ];
        let resolved = vec![
            res("@a", None, 10),
            res("@b", None, 20),
            res("@c", None, 30),
            res("@x", None, 99),
        ];
        let tree = build_annotated_tree(
            &files,
            &resolved,
            Some(1),
            &["backstage".to_string()],
        );
        let backstage = tree
            .root
            .children
            .get("backstage")
            .expect("backstage anchor still present");
        assert!(
            backstage.truncated.is_none(),
            "anchor itself is within budget"
        );
        let api = backstage.children.get("api").expect("api");
        let api_stats = api.truncated.as_ref().unwrap();
        assert_eq!(api_stats, &vec![(10u32, 1usize), (20u32, 1usize)]);

        let db = backstage.children.get("db").expect("db");
        let db_stats = db.truncated.as_ref().unwrap();
        assert_eq!(db_stats, &vec![(30u32, 1usize)]);

        // "other" is outside the anchor path → untouched.
        let other = tree.root.children.get("other").expect("other");
        assert!(other.truncated.is_none());
    }

    #[test]
    fn max_depth_at_anchor_collapses_the_anchor_itself() {
        // maxDepth=0 with a non-root anchor: the anchor's own subtree
        // is past the budget so it collapses in place. Second top-level
        // dir "other" keeps base empty so we can still target
        // "backstage" as a non-root anchor.
        let files = vec![
            "backstage/api/one.rs".to_string(),
            "backstage/db/two.rs".to_string(),
            "other/z.rs".to_string(),
        ];
        let resolved = vec![
            res("@a", None, 10),
            res("@b", None, 20),
            res("@x", None, 99),
        ];
        let tree = build_annotated_tree(
            &files,
            &resolved,
            Some(0),
            &["backstage".to_string()],
        );
        let backstage = tree
            .root
            .children
            .get("backstage")
            .expect("backstage anchor still present");
        let stats = backstage.truncated.as_ref().unwrap();
        assert_eq!(stats.len(), 2, "backstage subtree mixes 2 rules");
        // "other" was not part of the anchor path — untouched.
        let other = tree.root.children.get("other").expect("other");
        assert!(other.truncated.is_none());
    }

    #[test]
    fn max_depth_ignored_without_user_paths() {
        let files = vec![
            "app/mixed/one.rs".to_string(),
            "app/mixed/two.rs".to_string(),
            "other/z.rs".to_string(),
        ];
        let resolved =
            vec![res("@a", None, 10), res("@b", None, 20), res("@c", None, 30)];
        let tree = build_annotated_tree(&files, &resolved, Some(0), &[]);
        fn any_truncated(node: &TrieNode) -> bool {
            node.truncated.is_some()
                || node.children.values().any(any_truncated)
        }
        assert!(!any_truncated(&tree.root));
    }
}
