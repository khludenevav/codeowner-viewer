//! Implementation of the MCP `list_owners` tool: enumerate the distinct
//! owner handles present in the repo at HEAD. Small discovery helper
//! that lets an agent construct a valid `owners` filter for
//! `export_codeowners` without guessing.
//!
//! Output is intentionally minimal — just `{"owners": ["@t/a", ...]}`
//! sorted alphabetically. Unowned files do NOT contribute an entry.
//! For per-owner file counts see `owners_stats`.

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::app_config::AppConfigStore;

use super::{
    error::McpToolError,
    export_builder::{self, ExportFilters},
    repo_resolver,
    schema::{ListOwnersInput, ListOwnersOutput},
    tool_get_codeowners::HEAD_REF,
};

pub fn run(
    store: &Arc<AppConfigStore>,
    input: ListOwnersInput,
) -> Result<ListOwnersOutput, McpToolError> {
    let repo = repo_resolver::resolve(store, &input.repo)?;

    let payload =
        export_builder::build_export_payload(&repo, HEAD_REF, &ExportFilters::default())?;

    let mut owners: BTreeSet<String> = BTreeSet::new();
    for rule in payload.rules.values() {
        for owner in &rule.owners {
            owners.insert(owner.clone());
        }
    }

    Ok(ListOwnersOutput {
        owners: owners.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::export_builder::{ExportPayload, RulePayload, StatsPayload, SCHEMA_VERSION};
    use std::collections::BTreeMap;

    /// Aggregation logic factored out for testing without a git repo.
    fn aggregate(payload: &ExportPayload) -> Vec<String> {
        let mut owners: BTreeSet<String> = BTreeSet::new();
        for rule in payload.rules.values() {
            for owner in &rule.owners {
                owners.insert(owner.clone());
            }
        }
        owners.into_iter().collect()
    }

    fn rule(owners: &[&str]) -> RulePayload {
        RulePayload {
            owners: owners.iter().map(|s| s.to_string()).collect(),
            comment: None,
        }
    }

    fn payload_from_rules(rules: Vec<(&str, RulePayload)>) -> ExportPayload {
        let mut map: BTreeMap<String, RulePayload> = BTreeMap::new();
        for (k, v) in rules {
            map.insert(k.to_string(), v);
        }
        ExportPayload {
            schema: SCHEMA_VERSION.to_string(),
            rules: map,
            files: BTreeMap::new(),
            stats: StatsPayload {
                total_files: 0,
                owned_files: 0,
                unowned_files: 0,
                rule_count: 0,
                codeowners_file_path: "CODEOWNERS".to_string(),
                filtered: false,
                generated_at: "2026-09-06T18:15:28Z".to_string(),
            },
        }
    }

    #[test]
    fn dedupes_across_rules_and_sorts_alpha() {
        let payload = payload_from_rules(vec![
            ("10", rule(&["@t/b", "@t/a"])),
            ("20", rule(&["@t/a", "@t/c"])),
        ]);
        assert_eq!(aggregate(&payload), vec!["@t/a", "@t/b", "@t/c"]);
    }

    #[test]
    fn skips_unowned_rule() {
        let payload = payload_from_rules(vec![
            ("0", rule(&[])),
            ("10", rule(&["@t/a"])),
        ]);
        assert_eq!(aggregate(&payload), vec!["@t/a"]);
    }

    #[test]
    fn empty_when_no_owners() {
        let payload = payload_from_rules(vec![("0", rule(&[]))]);
        assert!(aggregate(&payload).is_empty());
    }
}
