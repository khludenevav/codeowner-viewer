//! Implementation of the MCP `owners_stats` tool: per-owner file
//! counts + repo-wide totals at HEAD.
//!
//! Complements `list_owners` (bare names): use this when you want to
//! rank owners by footprint before picking a filter for
//! `export_codeowners`. Files with N co-owners contribute +1 to each of
//! those N owners, so the sum of per-owner `files` may exceed
//! `totalFiles`.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use crate::app_config::AppConfigStore;

use super::{
    error::McpToolError,
    export_builder::{self, ExportFilters, ExportPayload, UNOWNED_RULE_ID},
    repo_resolver,
    schema::{OwnerStat, OwnersStatsInput, OwnersStatsOutput},
    tool_get_codeowners::HEAD_REF,
};

pub fn run(
    store: &Arc<AppConfigStore>,
    input: OwnersStatsInput,
) -> Result<OwnersStatsOutput, McpToolError> {
    let repo = repo_resolver::resolve(store, &input.repo)?;

    let payload =
        export_builder::build_export_payload(&repo, HEAD_REF, &ExportFilters::default())?;

    Ok(aggregate(&payload))
}

fn aggregate(payload: &ExportPayload) -> OwnersStatsOutput {
    // Files per rule id.
    let mut files_per_rule: HashMap<u32, usize> = HashMap::new();
    for rule_id in payload.files.values() {
        *files_per_rule.entry(*rule_id).or_insert(0) += 1;
    }

    // Distribute rule file counts across each owner listed on the rule.
    // BTreeMap gives alphabetical JSON key order.
    let mut owners: BTreeMap<String, OwnerStat> = BTreeMap::new();
    for (rule_key, rule) in &payload.rules {
        let rid: u32 = rule_key.parse().unwrap_or(UNOWNED_RULE_ID);
        if rid == UNOWNED_RULE_ID {
            continue;
        }
        let count = files_per_rule.get(&rid).copied().unwrap_or(0);
        if count == 0 {
            continue;
        }
        for owner in &rule.owners {
            owners
                .entry(owner.clone())
                .or_insert(OwnerStat { files: 0 })
                .files += count;
        }
    }

    OwnersStatsOutput {
        total_files: payload.stats.total_files,
        unowned_files: payload.stats.unowned_files,
        owners,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::export_builder::{RulePayload, StatsPayload, SCHEMA_VERSION};

    fn rule(owners: &[&str]) -> RulePayload {
        RulePayload {
            owners: owners.iter().map(|s| s.to_string()).collect(),
            comment: None,
        }
    }

    fn build(
        rules: Vec<(&str, RulePayload)>,
        files: Vec<(&str, u32)>,
        stats_total: usize,
        stats_unowned: usize,
    ) -> ExportPayload {
        let mut rmap: BTreeMap<String, RulePayload> = BTreeMap::new();
        for (k, v) in rules {
            rmap.insert(k.to_string(), v);
        }
        let mut fmap: BTreeMap<String, u32> = BTreeMap::new();
        for (k, v) in files {
            fmap.insert(k.to_string(), v);
        }
        ExportPayload {
            schema: SCHEMA_VERSION.to_string(),
            rules: rmap,
            files: fmap,
            stats: StatsPayload {
                total_files: stats_total,
                owned_files: stats_total - stats_unowned,
                unowned_files: stats_unowned,
                rule_count: 0,
                codeowners_file_path: "CODEOWNERS".to_string(),
                filtered: false,
                generated_at: "2026-09-06T18:15:28Z".to_string(),
            },
        }
    }

    #[test]
    fn counts_files_per_owner() {
        // Rule 10 → @t/a (3 files); Rule 20 → @t/b (1 file)
        let payload = build(
            vec![("10", rule(&["@t/a"])), ("20", rule(&["@t/b"]))],
            vec![("a.txt", 10), ("b.txt", 10), ("c.txt", 10), ("d.txt", 20)],
            4,
            0,
        );
        let out = aggregate(&payload);
        assert_eq!(out.total_files, 4);
        assert_eq!(out.unowned_files, 0);
        assert_eq!(out.owners.len(), 2);
        assert_eq!(out.owners["@t/a"].files, 3);
        assert_eq!(out.owners["@t/b"].files, 1);
    }

    #[test]
    fn co_owners_each_get_a_count_for_the_same_file() {
        let payload = build(
            vec![("10", rule(&["@t/a", "@t/b"]))],
            vec![("x.txt", 10), ("y.txt", 10)],
            2,
            0,
        );
        let out = aggregate(&payload);
        assert_eq!(out.owners.len(), 2);
        assert_eq!(out.owners["@t/a"].files, 2);
        assert_eq!(out.owners["@t/b"].files, 2);
    }

    #[test]
    fn unowned_files_are_not_attributed_to_any_owner() {
        let payload = build(
            vec![("0", rule(&[])), ("10", rule(&["@t/a"]))],
            vec![
                ("orphan1.txt", 0),
                ("orphan2.txt", 0),
                ("orphan3.txt", 0),
                ("owned.txt", 10),
            ],
            4,
            3,
        );
        let out = aggregate(&payload);
        assert_eq!(out.total_files, 4);
        assert_eq!(out.unowned_files, 3);
        assert_eq!(out.owners.len(), 1);
        assert_eq!(out.owners["@t/a"].files, 1);
    }

    #[test]
    fn owner_appearing_in_multiple_rules_sums_across_them() {
        let payload = build(
            vec![("10", rule(&["@t/a"])), ("20", rule(&["@t/a", "@t/b"]))],
            vec![("a.txt", 10), ("b.txt", 10), ("c.txt", 20)],
            3,
            0,
        );
        let out = aggregate(&payload);
        assert_eq!(out.owners["@t/a"].files, 3);
        assert_eq!(out.owners["@t/b"].files, 1);
    }

    #[test]
    fn serializes_as_owner_keyed_map_with_files_object() {
        let payload = build(
            vec![("10", rule(&["@t/a"]))],
            vec![("a.txt", 10)],
            1,
            0,
        );
        let out = aggregate(&payload);
        let json = serde_json::to_string(&out).unwrap();
        assert!(
            json.contains(r#""owners":{"@t/a":{"files":1}}"#),
            "expected owner-keyed map with files object, got: {json}"
        );
        // Confirm the timestamp field was dropped from the wire shape.
        assert!(
            !json.contains("generatedAt") && !json.contains("generated_at"),
            "did not expect a generation-timestamp field in owners_stats output, got: {json}"
        );
    }
}
