# Changelog

All notable changes to Codeowners viewer are documented in this file.

<!--
Release process:
- The version at the top must match the top-level `version` in `src-tauri/tauri.conf.json`.
- When you push a `v*` tag, `.github/workflows/publish.yml` reads the section for
  that version and uses it as the GitHub release body.
- Use `## <version>` to start a new entry and `-` bullets for the notes.
-->

## 0.17.1

- "Error in restarting app automatically" on update is expected.
- Updated all dependencies versions.
- Fixed update dialog scroll.

## 0.17.0

- Branch changes: added an "Uncommitted changes" toggle (on by default) that merges the working-tree diff (staged + unstaged + untracked) into the list of changed files. The toggle is disabled visually when a non-HEAD branch is selected but the user's preference is preserved and re-applied automatically when HEAD is picked again.
- MCP `get_codeowners`: response is now a compact DSL body (not JSON), with `responseMode` collapsed to `compact` (default) / `full`. Responses over 50 KB are truncated in-place with a `fullDumpPath:` header pointing at the untruncated dump. Log rows show response size. **Breaking:** the removed `responseMode: "normal"` value is no longer accepted.
- MCP `get_codeowners`: added optional `maxDepth` input to cap how deep the tree recurses below each entry of `paths` (depth 0 = the requested path itself). Subtrees past the budget are collapsed in place; uniform ones render as `dir/ <rule>` and mixed ones as `[id:count,…] TRUNCATED`. Same TRUNCATED shape is now emitted by the size guard, and includes per-rule file counts so agents can see the ownership breakdown of a hidden subtree at a glance.
- MCP `get_codeowners`: DSL body now uses 1-space indent per tree level (was 2). Trims byte size and helps deep monorepo paths stay under the 50 KB guard budget.
- MCP: added a second tool **`export_codeowners`** that dumps the full ownership map for a repo to a JSON file (`codeowners-export/v1` schema) and returns just the file path + counts. Intended for scripting — Python's stdlib parses it directly. Supports optional `owners[]` and `extensions[]` filters (OR within each list, AND between the two) and an optional `path` (absolute) that pins the dump to a caller-chosen location and overwrites any existing file; when omitted the dump lands in the OS temp dir and files matching `export-*.json` older than 7 days are pruned on each invocation. Includes `stats.generated_at`. Same JSON is now produced by the UI "Export to json..." button on the Repo Owners page, so the manual export and the tool export are byte-identical.
- MCP: added two discovery tools — **`list_owners`** (returns `{"owners": ["@team/a", ...]}` — the distinct owner handles present in the repo at HEAD, alphabetically sorted, unowned files excluded) and **`owners_stats`** (returns `{totalFiles, unownedFiles, owners: {"@team/a": {files: N}, ...}}` — per-owner file counts + repo-wide totals). Use them to pick a good `owners[]` filter for `export_codeowners` without guessing at handle strings. A file with N co-owners contributes +1 to each owner in `owners_stats`, so the sum may exceed `totalFiles`.
- Fixed a bug in the "Filter repo tree by owner" list where teams appeared twice (once as `@team` and once as `@team,`) for CODEOWNERS rules with 3+ owners. The owner splitter was stripping only the first comma from the joined string.

## 0.16.2

- "Error in restarting app automatically" on update is expected.
- Migrated to Tauri v2. Impact: security. You have to update before November 1st 2026, or auto next auto update will not work.

## 0.15.2

- Added an MCP server so local coding agents can query CODEOWNERS over the Model Context Protocol.
- The app now reacts to system theme changes at runtime.
- Fixed a repository id collision that could occur for some paths.

## 0.14.1

- Added support for multiple repositories
- x95 times faster calculations for the whole repo.
