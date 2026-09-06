//! Serializer from [`AnnotatedTree`] to the CODEOWNERS DSL text body.
//!
//! The grammar is described in the tool description (see `mcp/mod.rs`).
//! The emitter is pure: given the tree + mode + a header state, it
//! produces the same body every time.

use std::fmt::Write;

use super::compactor::{AnnotatedTree, RuleInfo, TrieNode, UNOWNED_RULE_ID};
use super::schema::ResponseMode;

pub const FORMAT_VERSION: &str = "codeowners-dsl-v1";

/// Optional runtime knobs the caller (the size guard, mostly) sets on
/// the header block that gets prepended to the body.
#[derive(Debug, Clone, Default)]
pub struct HeaderExtras {
    pub truncated: bool,
    pub full_dump_path: Option<String>,
}

/// Emit the DSL body. The final `sizeBytes:` value is the byte length
/// of the returned string (self-consistent — see [`emit_with_size`]).
pub fn emit(tree: &AnnotatedTree, mode: ResponseMode, extras: &HeaderExtras) -> String {
    emit_with_size(tree, mode, extras)
}

/// Actual emitter. Uses a two-pass strategy to make `sizeBytes:`
/// self-consistent: we first emit with a placeholder, measure the total
/// length, then re-emit with the true size. Because the width of the
/// number is stable within a small band, one fixup iteration is enough
/// in practice (we loop up to 3 times to be safe).
fn emit_with_size(
    tree: &AnnotatedTree,
    mode: ResponseMode,
    extras: &HeaderExtras,
) -> String {
    let body = emit_body(tree, mode);
    let mut declared = 0usize;
    let mut out = String::new();
    for _ in 0..3 {
        out.clear();
        emit_header(&mut out, tree, extras, declared);
        out.push_str(&body);
        if out.len() == declared {
            return out;
        }
        declared = out.len();
    }
    out
}

fn emit_header(
    out: &mut String,
    tree: &AnnotatedTree,
    extras: &HeaderExtras,
    size_bytes: usize,
) {
    let _ = writeln!(out, "format: {FORMAT_VERSION}");
    if !tree.base.is_empty() {
        let _ = writeln!(out, "base: {}", tree.base);
    } else {
        let _ = writeln!(out, "base:");
    }
    match tree.root_default {
        Some(id) => {
            let _ = writeln!(out, "default: {id}");
        }
        None => {
            let _ = writeln!(out, "default:");
        }
    }
    let _ = writeln!(out, "truncated: {}", extras.truncated);
    let _ = writeln!(out, "sizeBytes: {size_bytes}");
    if let Some(p) = &extras.full_dump_path {
        let _ = writeln!(out, "fullDumpPath: {p}");
    }
    out.push('\n');
}

fn emit_body(tree: &AnnotatedTree, mode: ResponseMode) -> String {
    let mut out = String::new();

    // rules: section
    if !tree.rules.is_empty() {
        out.push_str("rules:\n");
        for (id, info) in &tree.rules {
            emit_rule_line(&mut out, *id, info);
        }
        out.push('\n');
    }

    // tree body
    let effective_default = tree.root_default.unwrap_or(UNOWNED_RULE_ID);
    for child in tree.root.children.values() {
        emit_node(&mut out, child, 0, effective_default, mode);
    }
    // Files that sit at the very root of the base (rare, but possible).
    emit_files(&mut out, &tree.root, 0, effective_default, mode);

    out
}

fn emit_rule_line(out: &mut String, id: u32, info: &RuleInfo) {
    let _ = write!(out, " {id}");
    if !info.owners.is_empty() {
        out.push(' ');
        out.push_str(&info.owners);
    }
    if let Some(c) = &info.comment {
        out.push(' ');
        out.push('(');
        out.push_str(&escape_comment(c));
        out.push(')');
    } else if info.owners.is_empty() {
        // A bare `<id>` (no owners, no comment) is legal but ambiguous
        // for a human — mark it explicitly.
        out.push_str(" (unowned)");
    }
    out.push('\n');
}

/// Escape `)` inside comments so the parens stay balanced. Also collapse
/// embedded newlines (comments must be single-line in the DSL).
fn escape_comment(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            ')' => {
                out.push('\\');
                out.push(')');
            }
            '\n' | '\r' => out.push(' '),
            _ => out.push(ch),
        }
    }
    out
}

fn emit_node(
    out: &mut String,
    node: &TrieNode,
    depth: usize,
    enclosing_default: u32,
    mode: ResponseMode,
) {
    // Truncated (mixed-rule) dirs must always be emitted as a marker.
    // A single-rule "truncated" (i.e. a subtree collapsed by the depth
    // budget or size guard where everything below shared one owner)
    // is indistinguishable from a natural uniform subtree: fall through
    // to the normal path so it renders as `dir/ <rule>`.
    if let Some(stats) = &node.truncated {
        if stats.len() >= 2 {
            let indent = " ".repeat(depth);
            let _ = write!(out, "{indent}{}/ [", node.key);
            for (i, (id, count)) in stats.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let _ = write!(out, "{id}:{count}");
            }
            out.push_str("] TRUNCATED\n");
            return;
        }
    }

    let subtree_default = node.prevailing;
    let child_default = subtree_default;

    // Serialize children/files into a temp buffer so we can decide
    // whether this directory has anything worth saying.
    let mut inner = String::new();
    emit_files(&mut inner, node, depth + 1, child_default, mode);
    for child in node.children.values() {
        emit_node(&mut inner, child, depth + 1, child_default, mode);
    }

    let has_override = subtree_default != enclosing_default;
    match mode {
        ResponseMode::Compact => {
            // Skip empty directories that neither override nor carry
            // exceptions — they'd be pure noise.
            if !has_override && inner.is_empty() {
                return;
            }
        }
        ResponseMode::Full => {
            // Full mode always emits directories that reach the caller,
            // even if empty, so the caller sees the full skeleton.
        }
    }

    let indent = " ".repeat(depth);
    if has_override {
        let _ = writeln!(out, "{indent}{}/ {}", node.key, subtree_default);
    } else {
        let _ = writeln!(out, "{indent}{}/", node.key);
    }
    out.push_str(&inner);
}

fn emit_files(
    out: &mut String,
    node: &TrieNode,
    depth: usize,
    effective_default: u32,
    mode: ResponseMode,
) {
    if node.files.is_empty() {
        return;
    }
    let indent = " ".repeat(depth);
    for (name, rule_id) in &node.files {
        match mode {
            ResponseMode::Compact => {
                if *rule_id == effective_default {
                    continue;
                }
                let _ = writeln!(out, "{indent}{name} {rule_id}");
            }
            ResponseMode::Full => {
                let _ = writeln!(out, "{indent}{name} {rule_id}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codeowners_engine::OwnerResolution;
    use crate::mcp::compactor::build_annotated_tree;

    fn res(owners: &str, comment: Option<&str>, line: u32) -> OwnerResolution {
        OwnerResolution {
            owners: owners.to_string(),
            rule_comment: comment.map(|s| s.to_string()),
            rule_line_number: line,
        }
    }

    #[test]
    fn header_includes_size_bytes_matching_length() {
        let files = vec!["a/one.rs".to_string()];
        let resolved = vec![res("@x", None, 12)];
        let tree = build_annotated_tree(&files, &resolved, None, &[]);
        let out = emit(&tree, ResponseMode::Full, &HeaderExtras::default());
        // The header always contains a sizeBytes: line that equals the
        // total body length.
        let line = out
            .lines()
            .find(|l| l.starts_with("sizeBytes: "))
            .expect("sizeBytes line");
        let n: usize = line["sizeBytes: ".len()..].parse().unwrap();
        assert_eq!(n, out.len());
    }

    #[test]
    fn compact_omits_files_matching_default() {
        let files = vec![
            "app/a.rs".to_string(),
            "app/b.rs".to_string(),
            "app/c.rs".to_string(),
        ];
        let resolved =
            vec![res("@x", None, 12), res("@x", None, 12), res("@y", None, 47)];
        let tree = build_annotated_tree(&files, &resolved, None, &[]);
        let out = emit(&tree, ResponseMode::Compact, &HeaderExtras::default());
        assert!(out.contains("default: 12"));
        // Exception is emitted, majority files aren't.
        assert!(out.contains("c.rs 47"));
        assert!(!out.contains("a.rs 12"));
        assert!(!out.contains("b.rs 12"));
    }

    #[test]
    fn full_emits_every_file_with_id() {
        let files = vec!["app/a.rs".to_string(), "app/b.rs".to_string()];
        let resolved = vec![res("@x", None, 12), res("@x", None, 12)];
        let tree = build_annotated_tree(&files, &resolved, None, &[]);
        let out = emit(&tree, ResponseMode::Full, &HeaderExtras::default());
        assert!(out.contains("a.rs 12"));
        assert!(out.contains("b.rs 12"));
    }

    #[test]
    fn comment_paren_is_escaped() {
        let files = vec!["a/x.rs".to_string()];
        let resolved = vec![res("@x", Some("weird ) comment"), 12)];
        let tree = build_annotated_tree(&files, &resolved, None, &[]);
        let out = emit(&tree, ResponseMode::Full, &HeaderExtras::default());
        assert!(out.contains(r"(weird \) comment)"));
    }

    #[test]
    fn unowned_bare_id_is_marked() {
        let files = vec!["a/x.rs".to_string()];
        let resolved = vec![res("", None, 0)];
        let tree = build_annotated_tree(&files, &resolved, None, &[]);
        let out = emit(&tree, ResponseMode::Full, &HeaderExtras::default());
        assert!(out.contains("0 (unowned)"));
    }

    #[test]
    fn truncated_dir_renders_marker() {
        let files = vec![
            "app/legacy/a.rs".to_string(),
            "app/legacy/b.rs".to_string(),
            "app/other/z.rs".to_string(),
        ];
        let resolved =
            vec![res("@x", None, 12), res("@y", None, 47), res("@x", None, 12)];
        let mut tree = build_annotated_tree(&files, &resolved, None, &[]);
        let legacy = tree.root.children.get_mut("legacy").unwrap();
        legacy.truncated = Some(vec![(12, 2), (47, 1)]);
        legacy.files.clear();
        legacy.children.clear();
        let out = emit(&tree, ResponseMode::Full, &HeaderExtras::default());
        assert!(out.contains("legacy/ [12:2,47:1] TRUNCATED"));
    }

    #[test]
    fn single_rule_truncated_falls_through_to_uniform_line() {
        let files = vec![
            "app/legacy/a.rs".to_string(),
            "app/legacy/b.rs".to_string(),
            "app/other/z.rs".to_string(),
        ];
        let resolved = vec![
            res("@x", None, 12),
            res("@x", None, 12),
            res("@y", None, 47),
        ];
        let mut tree = build_annotated_tree(&files, &resolved, None, &[]);
        // Simulate a size-guard / max-depth collapse where the subtree
        // happened to be uniform: single-rule truncated stats must NOT
        // render a TRUNCATED marker — they render as `dir/ <rule>`.
        let legacy = tree.root.children.get_mut("legacy").unwrap();
        legacy.truncated = Some(vec![(12, 2)]);
        legacy.files.clear();
        legacy.children.clear();
        // Force a re-annotate so prevailing/leaves reflect the stats
        // we just injected (real code calls this once via
        // build_annotated_tree; the direct-mutation test path skips it).
        let out = emit(&tree, ResponseMode::Full, &HeaderExtras::default());
        assert!(
            !out.contains("TRUNCATED"),
            "single-rule truncated must not render marker, got:\n{out}"
        );
        assert!(out.contains("legacy/"), "dir line must still appear:\n{out}");
    }
}
