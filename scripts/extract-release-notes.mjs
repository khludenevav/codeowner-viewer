#!/usr/bin/env node
// Extracts the release notes for a single version from CHANGELOG.md.
//
// Usage:
//   node scripts/extract-release-notes.mjs [version]
//
// If `version` is omitted, it is read from `package.version` in
// src-tauri/tauri.conf.json — that's what the publish workflow does.
//
// The matching section is everything between a heading `## <version>` and the
// next `## ` heading (or end of file). Leading/trailing blank lines are trimmed.
// Exits with a non-zero code and a clear message if no matching section exists.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const changelogPath = resolve(repoRoot, 'CHANGELOG.md');
const tauriConfPath = resolve(repoRoot, 'src-tauri/tauri.conf.json');

function readVersionFromTauriConf() {
  const raw = readFileSync(tauriConfPath, 'utf8');
  const parsed = JSON.parse(raw);
  const version = parsed?.package?.version;
  if (typeof version !== 'string' || version.length === 0) {
    throw new Error(`Could not read package.version from ${tauriConfPath}`);
  }
  return version;
}

function extractSection(changelog, version) {
  const lines = changelog.split(/\r?\n/);
  const collected = [];
  let inside = false;

  for (const line of lines) {
    const versionHeading = /^##\s+(\S+)/.exec(line);
    if (versionHeading) {
      if (inside) break; // next version reached
      if (versionHeading[1] === version) {
        inside = true;
        continue;
      }
    }
    if (inside) collected.push(line);
  }

  if (!inside) return null;

  // Trim leading/trailing blank lines.
  while (collected.length > 0 && collected[0].trim() === '') collected.shift();
  while (collected.length > 0 && collected[collected.length - 1].trim() === '') collected.pop();

  return collected.join('\n');
}

const version = process.argv[2] ?? readVersionFromTauriConf();
const changelog = readFileSync(changelogPath, 'utf8');
const notes = extractSection(changelog, version);

if (notes === null || notes.length === 0) {
  console.error(
    `::error::No CHANGELOG.md entry found for version ${version}. ` +
      `Add a '## ${version}' section before pushing the tag.`,
  );
  process.exit(1);
}

process.stdout.write(notes);
if (!notes.endsWith('\n')) process.stdout.write('\n');
