## Getting Started

Desktop application for exploring `CODEOWNERS` — the file that declares who owns
what in a git repository.

## Installation

- Download app installation file with `.dmg` extension [from release page](https://github.com/khludenevav/codeowner-viewer/releases/latest). Note, only M1, M2, M3.. processors supported.
- Install it as usual app
- Execute in any console `xattr -dr com.apple.quarantine /Applications/Codeowners\ viewer.app`.
  It required because app is not signed, because Apple asks to pay 99$ per year for signing: https://disable-gatekeeper.github.io/
- Done. You can launch app

App supports auto update if new version available on application launch. To check updates relaunch it. Update doesn't require to enter any new console commands. Just couple "Yes" clicks.

### Capabilities

- **Branch changes** — for any git branch, list the codeowners of the files that
  differ from `main`, grouped by team. A toggle folds in uncommitted working-tree
  changes so you can see who will need to review before you push.
- **File owners** — pick any file in the repo and see which CODEOWNERS rule
  matches, which teams own it, and the rule's inline comment (e.g. `#!required`).
- **Repo owners** — browse the whole repository as an ownership tree; every
  folder shows its resolved owners, and you can filter the tree by team.
- **Multi-repo tabs** — open several repositories side-by-side as tabs; each tab
  remembers its own selected branch, filters, and view.
- **Export to JSON** — dump the full ownership map to a
  JSON file. Filterable by team and file extension.
- **MCP server** — exposes an MCP server so local
  coding agents can query codeowners directly. All requests are logged in the app UI
  so you can see what your agent asked.
- **Auto-update** — checks for a new release on every launch; updates are a
  couple of clicks.

## Used technologies

Main framework: [Tauri v2](https://tauri.app/) (Rust backend + web frontend
bundled as a native desktop app).

### Frontend

- Language: TypeScript + React 18
- Build tool: [Vite](https://vitejs.dev/)
- File-system router: [`@tanstack/react-router`](https://tanstack.com/router)
- Server/cache state: [`@tanstack/react-query`](https://tanstack.com/query)
- Virtualized lists: `@tanstack/react-virtual`
- CSS framework: [Tailwind CSS](https://tailwindcss.com/) with
  `tailwindcss-animate`, `tailwind-merge`, `class-variance-authority`, `clsx`
- Headless UI components: [`shadcn/ui`](https://ui.shadcn.com/) on top of
  [Radix UI](https://www.radix-ui.com/) primitives (dialog, popover, checkbox,
  dropdown, tooltip, navigation menu)
- Command palette: [`cmdk`](https://cmdk.paco.me/)
- Toasts: [`sonner`](https://sonner.emilkowal.ski/)
- Theme switching: [`next-themes`](https://github.com/pacocoursey/next-themes)
- Icons: [`lucide-react`](https://lucide.dev/) + `file-extension-icon-js`
- Date formatting: [`dayjs`](https://day.js.org/)
- Lint/format: Oxlint, oxfmt

### Backend

- Tauri v2
- Serialization: `serde`, `serde_json`, `toml_edit`
- CODEOWNERS parsing (no external packages)
- Parallelism: `rayon`, `ahash`, `tokio` (multi-thread)
- MCP server: [`rmcp`](https://crates.io/crates/rmcp) with the streamable-HTTP
  transport, exposed over `axum` + `hyper` + `tower`
- Schema for MCP tools: `schemars`
- Timestamps: `chrono`
- Logging: `tracing` + `tracing-subscriber`
- Error handling: `anyhow`, `thiserror`
- Tests: `pretty_assertions`, `tempfile`
