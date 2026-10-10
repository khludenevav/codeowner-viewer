#!/usr/bin/env bash
# Shows which files of a repo change owners between this checkout's parser and a base ref's parser.
#
#   scripts/compare-codeowners.sh <repo> [base-ref=main] [out-dir=/tmp/compare-codeowners]
#
# Writes <out-dir>/before.tsv, after.tsv (path, rule line, owners) and diff.tsv, then prints
# the changes grouped by "old rule -> new rule". Check a dump against git with
# scripts/check-codeowners-with-git.py.
set -euo pipefail

repo=$(cd "$1" && pwd)
base=${2:-main}
out=${3:-/tmp/compare-codeowners}
root=$(cd "$(dirname "$0")/.." && pwd)
wt="$out/base-worktree"

mkdir -p "$out"
git -C "$root" worktree remove --force "$wt" 2>/dev/null || true
git -C "$root" worktree add -q --detach "$wt" "$base"
trap 'git -C "$root" worktree remove --force "$wt"' EXIT

mkdir -p "$wt/src-tauri/examples"
cp "$root/src-tauri/examples/dump_owners.rs" "$wt/src-tauri/examples/"

dump() { # <checkout> <target-dir> <out-file>
  CARGO_TARGET_DIR="$2" cargo run -q --release --manifest-path "$1/src-tauri/Cargo.toml" \
    --example dump_owners -- "$repo" >"$3"
}
dump "$root" "$root/src-tauri/target" "$out/after.tsv"
dump "$wt" "$root/src-tauri/target/compare-base" "$out/before.tsv"

paste "$out/before.tsv" "$out/after.tsv" |
  awk -F'\t' '$1 != $4 { print "file lists differ at " $1 > "/dev/stderr"; exit 1 } $2 != $5' >"$out/diff.tsv"

echo "files: $(wc -l <"$out/after.tsv"), changed: $(wc -l <"$out/diff.tsv") (base: $base)"
awk -F'\t' '{ print $2 " -> " $5 }' "$out/diff.tsv" | sort | uniq -c | sort -rn
echo "details: $out/diff.tsv (path, old line, old owners, path, new line, new owners)"
