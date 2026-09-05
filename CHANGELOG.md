# Changelog

All notable changes to Codeowners viewer are documented in this file.

<!--
Release process:
- The version at the top must match `package.version` in `src-tauri/tauri.conf.json`.
- When you push a `v*` tag, `.github/workflows/publish.yml` reads the section for
  that version and uses it as the GitHub release body.
- Use `## <version>` to start a new entry and `-` bullets for the notes.
-->

## 0.15.1

- Added an MCP server so local coding agents can query CODEOWNERS over the Model Context Protocol.
- The app now reacts to system theme changes at runtime.
- Fixed a repository id collision that could occur for some paths.

## 0.14.1

- Added support for multiple repositories
- x95 times faster calculations for the whole repo.
