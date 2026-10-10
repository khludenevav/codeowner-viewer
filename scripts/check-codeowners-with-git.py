#!/usr/bin/env python3
"""Checks a dump_owners result against git's own gitignore matcher (case-sensitive).

    python3 -I scripts/check-codeowners-with-git.py <CODEOWNERS file> <dump.tsv>

For every file the dump says rule L wins. Then git must match the file with
rules L and later, and must not match it with rules after L. A rule matching a
parent folder matches the file in both CODEOWNERS and gitignore, so this holds
even though `git check-ignore -v` itself reports the parent's rule.
Prints every disagreement and a summary line. Takes ~10 min on 190k files / 5.5k rules.

Known false alarm: `dir/*` in gitignore also matches sub-folders (and so everything
inside them); GitHub CODEOWNERS says `docs/*` does not own `docs/a/b.md`. So
"a later rule matches" with a `.../*` rule on a nested file means the dump is right.
"""
import collections
import os
import subprocess
import sys
import tempfile

codeowners, dump = sys.argv[1], sys.argv[2]
# Keep line numbers: pattern only, comments and blank lines become blank.
rules = []
for line in open(codeowners):
    s = line.strip()
    rules.append("" if not s or s.startswith("#") else s.split()[0])

files_by_rule = collections.defaultdict(list)
for row in open(dump):
    path, rule_line, _ = row.rstrip("\n").split("\t")
    files_by_rule[int(rule_line)].append(path)

work = tempfile.mkdtemp()
subprocess.run(["git", "init", "-q", work], check=True)
ignore_file = os.path.join(work, "rules.gitignore")


def matched(files, min_line):
    with open(ignore_file, "w") as f:
        f.write("\n".join(r if n >= min_line else "" for n, r in enumerate(rules, 1)))
    out = subprocess.run(
        ["git", "-C", work, "-c", "core.excludesFile=" + ignore_file, "-c", "core.ignoreCase=false",
         "check-ignore", "--no-index", "--stdin"],
        input="\n".join(files) + "\n", capture_output=True, text=True,
    ).stdout
    return set(out.splitlines())


bad = 0
for rule_line, files in sorted(files_by_rule.items()):
    if rule_line > 0:  # 0 = no rule matched
        for path in sorted(set(files) - matched(files, rule_line)):
            bad += 1
            print(f"rule {rule_line} does not match {path}", flush=True)
    for path in sorted(matched(files, rule_line + 1)):
        bad += 1
        print(f"rule {rule_line} chosen, but a later rule matches {path}", flush=True)

total = sum(map(len, files_by_rule.values()))
print(f"files: {total}, rules used: {len(files_by_rule)}, disagreements: {bad}")
sys.exit(1 if bad else 0)
