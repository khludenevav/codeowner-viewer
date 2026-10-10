# AGENTS.md

Tauri v2 desktop app (Rust + React) that shows CODEOWNERS ownership of a git repo
and runs a local MCP server (127.0.0.1:47821) with the same data.

## Map

- `src-tauri/src/codeowners_file_parser.rs` — CODEOWNERS parsing and pattern matching (GitHub semantics).
- `src-tauri/src/codeowners_engine.rs` — git file listing, batch owner resolution.
- `src-tauri/src/lib.rs` — Tauri commands used by the UI.
- `src-tauri/src/mcp/` — MCP server; one `tool_*.rs` per tool, `repo_resolver.rs` finds the CODEOWNERS file.
- `src-tauri/src/agent_installer/` — registers the MCP server in Claude, VS Code-family and Codex configs.
- `src-tauri/src/app_config.rs` — app config store.
- `src/routes/` — UI pages (TanStack Router, `routeTree.gen.ts` is generated); `src/mcp/` — MCP page.
- `scripts/compare-codeowners.sh <repo> [base-ref=main]` — dumps owners of every file with this
  checkout and with a base ref, groups changed files by rule. Run it after any parser/engine change.
- `scripts/check-codeowners-with-git.py <CODEOWNERS> <dump.tsv>` — checks a dump against git's matcher.
- `src-tauri/examples/dump_owners.rs` — the dump both scripts use.

## Commands

- `pnpm test` — Rust tests (no frontend test runner yet).
- `pnpm lint`, `pnpm format:check`, `npx tsc -b` — frontend checks.
- `pnpm build-app` — run the app in dev mode.
