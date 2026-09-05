//! Codeowners provides interfaces for resolving owners of paths within code
//! repositories using
//! Github [CODEOWNERS](https://help.github.com/articles/about-codeowners/)
//! files
//!
//! # Examples
//!
//! Typical use involves resolving a CODEOWNERS file, parsing it,
//! then querying target paths
//!
//! ```no_run
//! extern crate codeowners;
//! use std::env;
//!
//! fn main() {
//!   if let (Some(owners_file), Some(path)) =
//!      (env::args().nth(1), env::args().nth(2)) {
//!      let owners = codeowners::from_path(owners_file);
//!      match owners.of(&path) {
//!        None => println!("{} is up for adoption", path),
//!        Some(owners) => {
//!           for owner in owners {
//!             println!("{}", owner);
//!           }
//!        }
//!      }
//!   }
//! }
//! ```
#![allow(missing_docs)]

use glob::Pattern;
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    fmt,
    io::{BufRead, BufReader, Read},
    path::Path,
    str::FromStr,
};

/// Various types of owners
///
/// Owners supports parsing from strings as well as displaying as strings
///
/// # Examples
///
/// ```rust
/// let raw = "@org/team";
/// assert_eq!(
///   raw.parse::<codeowners::Owner>().unwrap().to_string(),
///   raw
/// );
/// ```
#[derive(Debug, PartialEq)]
pub enum Owner {
    /// Owner in the form @username
    Username(String),
    /// Owner in the form @org/Team
    Team(String),
    /// Owner in the form user@domain.com
    Email(String),
}

impl fmt::Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let inner = match *self {
            Owner::Username(ref u) => u,
            Owner::Team(ref t) => t,
            Owner::Email(ref e) => e,
        };
        f.write_str(inner.as_str())
    }
}

impl FromStr for Owner {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref TEAM: Regex = Regex::new(r"^@\S+/\S+").unwrap();
            static ref USERNAME: Regex = Regex::new(r"^@\S+").unwrap();
            static ref EMAIL: Regex = Regex::new(r"^\S+@\S+").unwrap();
        }

        if TEAM.is_match(s) {
            Ok(Owner::Team(s.into()))
        } else if USERNAME.is_match(s) {
            Ok(Owner::Username(s.into()))
        } else if EMAIL.is_match(s) {
            Ok(Owner::Email(s.into()))
        } else {
            Err(String::from("not an owner"))
        }
    }
}

/// Mappings of owners to path patterns
#[derive(Debug)]
pub struct Owners {
    paths: Vec<(Pattern, Vec<Owner>, Option<String>)>,
    /// Precomputed per-rule metadata that never changes for the lifetime of
    /// `Owners`. Stored in a parallel Vec (same length + same order as
    /// `paths`) to keep the on-disk / test-visible shape of `paths` stable.
    meta: Vec<RuleMeta>,
}

/// Per-rule precomputed data. `glob::MatchOptions` is `Copy` (three bools) so
/// this whole struct is trivially `Copy` too.
#[derive(Debug, Clone, Copy)]
struct RuleMeta {
    options: glob::MatchOptions,
    ends_with_slash_star: bool,
    /// 1-based line number in the source CODEOWNERS file. Used by MCP full
    /// response mode. Populated by `from_reader`; the tests-only constructor
    /// leaves this at 0.
    line_number: u32,
}

impl PartialEq for Owners {
    /// Only compares `paths`. `meta` is fully derived from `paths`, so
    /// comparing it would be redundant, and skipping it lets tests
    /// construct `Owners` without recomputing the metadata.
    fn eq(&self, other: &Self) -> bool {
        self.paths == other.paths
    }
}

impl Owners {
    /// Resolve a list of owners matching a given path
    pub fn of<P>(&self, path: P) -> Option<&Vec<Owner>>
    where
        P: AsRef<Path>,
    {
        let path_ref = path.as_ref();
        for (i, (pattern, owners, _)) in self.paths.iter().enumerate() {
            let meta = self.meta[i];
            if pattern.matches_path_with(path_ref, meta.options) {
                return Some(owners);
            }
            // this pattern is only meant to match direct children
            if meta.ends_with_slash_star {
                continue;
            }
            // case of implied owned children:
            // `foo/bar @owner` should indicate that `foo/bar/baz.rs` is
            // owned by `@owner`
            let mut p = path_ref;
            while let Some(parent) = p.parent() {
                if pattern.matches_path_with(parent, meta.options) {
                    return Some(owners);
                }
                p = parent;
            }
        }
        None
    }

    /// Resolve the inline comment (e.g. `#!required`) for the CODEOWNERS rule matching a given path.
    /// Returns `None` if no rule matches or the matching rule has no `#!` comment.
    pub fn comment_of<P>(&self, path: P) -> Option<&str>
    where
        P: AsRef<Path>,
    {
        let path_ref = path.as_ref();
        for (i, (pattern, _, comment)) in self.paths.iter().enumerate() {
            let meta = self.meta[i];
            let matches = if pattern.matches_path_with(path_ref, meta.options) {
                true
            } else if meta.ends_with_slash_star {
                false
            } else {
                let mut p = path_ref;
                let mut found = false;
                while let Some(parent) = p.parent() {
                    if pattern.matches_path_with(parent, meta.options) {
                        found = true;
                        break;
                    }
                    p = parent;
                }
                found
            };
            if matches {
                return comment.as_deref();
            }
        }
        None
    }

    /// Resolve owners **and** the inline `#!` comment in a single pattern
    /// scan. Returns `(owners, comment)` for the first rule that matches
    /// `path`, or `(None, None)` if no rule matches.
    ///
    /// Semantically equivalent to calling `of(path)` and `comment_of(path)`
    /// back-to-back, but only walks the pattern list (and parent chain)
    /// once — matters for the "changed files in a branch" code path in
    /// [`crate::get_changed_codeowners_for_branch`] which asks for both
    /// pieces of every file.
    pub fn of_with_comment<P>(&self, path: P) -> (Option<&Vec<Owner>>, Option<&str>)
    where
        P: AsRef<Path>,
    {
        let path_ref = path.as_ref();
        for (i, (pattern, owners, comment)) in self.paths.iter().enumerate() {
            let meta = self.meta[i];
            let matches = if pattern.matches_path_with(path_ref, meta.options) {
                true
            } else if meta.ends_with_slash_star {
                false
            } else {
                let mut p = path_ref;
                let mut found = false;
                while let Some(parent) = p.parent() {
                    if pattern.matches_path_with(parent, meta.options) {
                        found = true;
                        break;
                    }
                    p = parent;
                }
                found
            };
            if matches {
                return (Some(owners), comment.as_deref());
            }
        }
        (None, None)
    }

    /// Return each rule's owners rendered as a single comma-separated string.
    /// The returned vector is indexed by rule index (as returned by
    /// [`Self::of_index_with_ancestor`] / [`Self::direct_match_index_at`]).
    pub fn owner_strings(&self) -> Vec<String> {
        self.paths
            .iter()
            .map(|(_, owners, _)| {
                owners
                    .iter()
                    .map(|owner| format!("{owner}"))
                    .collect::<Vec<String>>()
                    .join(", ")
            })
            .collect()
    }

    /// Bulk / fast-path helper: index of the first rule that matches `dir`
    /// directly, considering only rules that do NOT end with `/*` (those are
    /// direct-child-only rules and are excluded from the ancestor walk in
    /// [`Self::of`]).
    ///
    /// This is the per-directory piece used to build the ancestor-index
    /// memoization cache when resolving owners for many files at once.
    pub fn direct_match_index_at(&self, dir: &Path) -> Option<usize> {
        for (i, (pattern, _, _)) in self.paths.iter().enumerate() {
            let meta = self.meta[i];
            if meta.ends_with_slash_star {
                continue;
            }
            if pattern.matches_path_with(dir, meta.options) {
                return Some(i);
            }
        }
        None
    }

    /// Bulk / fast-path helper: index of the first rule that matches file
    /// `path`, given `ancestor_index` — the precomputed "first rule index
    /// that ancestor-matches `path.parent()`", produced by combining
    /// [`Self::direct_match_index_at`] over the parent chain.
    ///
    /// Semantically equivalent to [`Self::of`], but avoids re-walking the
    /// parent chain per file: the caller already computed the ancestor
    /// result once per unique parent directory.
    pub fn of_index_with_ancestor(
        &self,
        path: &Path,
        ancestor_index: Option<usize>,
    ) -> Option<usize> {
        // We only need to check direct matches for rule indices strictly
        // smaller than the ancestor result, because any hit at or past
        // `ancestor_index` is already dominated by (or equal to) the
        // ancestor match, which we return unchanged.
        let upper = ancestor_index.unwrap_or(self.paths.len());
        for i in 0..upper {
            let (pattern, _, _) = &self.paths[i];
            if pattern.matches_path_with(path, self.meta[i].options) {
                return Some(i);
            }
        }
        // Fall back to the ancestor result. If it's `None`, we also need to
        // check the tail of the pattern list for a direct match (upper was
        // set to `self.paths.len()` above, so the loop already did that and
        // did not find one).
        ancestor_index
    }

    /// Total number of rules parsed from the CODEOWNERS file.
    pub fn rule_count(&self) -> usize {
        self.paths.len()
    }

    /// Raw inline comment (including the leading `#`) recorded for rule
    /// index `i`, or `None` if the rule had no inline comment. `None` is
    /// returned for out-of-range indices too.
    pub fn comment_at(&self, i: usize) -> Option<&str> {
        self.paths
            .get(i)
            .and_then(|(_, _, c)| c.as_deref())
    }

    /// 1-based line number in the source CODEOWNERS file that produced rule
    /// index `i`. Returns 0 for out-of-range indices or rules constructed
    /// without line-number tracking (tests-only path).
    pub fn line_number_at(&self, i: usize) -> u32 {
        self.meta.get(i).map(|m| m.line_number).unwrap_or(0)
    }
}

/// Parse a CODEOWNERS file from some readable source
/// This format is defined in
/// [Github's documentation](https://help.github.com/articles/about-codeowners/)
/// The syntax is uses gitgnore
/// [patterns](https://www.kernel.org/pub/software/scm/git/docs/gitignore.html#_pattern_format)
/// followed by an identifier for an owner. More information can be found
/// [here](https://help.github.com/articles/about-codeowners/#codeowners-syntax)
pub fn from_reader<R>(read: R) -> Owners
where
    R: Read,
{
    let (mut paths, mut line_numbers): (
        Vec<(Pattern, Vec<Owner>, Option<String>)>,
        Vec<u32>,
    ) = BufReader::new(read)
        .lines()
        .filter_map(Result::ok)
        .enumerate()
        // Preserve 1-based source line number for MCP full response mode.
        .map(|(idx, line)| ((idx as u32) + 1, line))
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
        .fold(
            (Vec::new(), Vec::new()),
            |(mut paths, mut line_numbers), (line_no, line)| {
                // Extract any inline `#…` comment from the rule line.
                // Consumers that only want `#!`-style comments (existing UI)
                // filter at the call site.
                let comment = line.find('#').map(|pos| line[pos..].trim().to_string());
                let line_content = match line.find('#') {
                    Some(pos) => &line[..pos],
                    None => line.as_str(),
                };
                let mut elements = line_content.split_whitespace();
                if let Some(pattern) = elements.next() {
                    let owners = elements.fold(Vec::new(), |mut result, owner| {
                        if let Ok(owner) = owner.parse() {
                            result.push(owner)
                        }
                        result
                    });
                    paths.push((make_pattern(pattern), owners, comment));
                    line_numbers.push(line_no);
                }
                (paths, line_numbers)
            },
        );
    // last match takes precedence
    paths.reverse();
    line_numbers.reverse();
    let meta = build_meta(&paths, &line_numbers);
    Owners { paths, meta }
}

/// Compute the per-rule `RuleMeta` derived from `paths`. Kept as a free
/// function so both `from_reader` and the tests-only constructor can share it.
/// `line_numbers` is a parallel Vec (same length + same order as `paths`);
/// pass an empty slice for tests-only constructors that don't care about
/// line numbers — meta entries will get `line_number = 0` in that case.
fn build_meta(
    paths: &[(Pattern, Vec<Owner>, Option<String>)],
    line_numbers: &[u32],
) -> Vec<RuleMeta> {
    paths
        .iter()
        .enumerate()
        .map(|(i, (pattern, _, _))| {
            let s = pattern.as_str();
            RuleMeta {
                options: glob::MatchOptions {
                    case_sensitive: false,
                    require_literal_separator: s.contains('/'),
                    require_literal_leading_dot: false,
                },
                ends_with_slash_star: s.ends_with("/*"),
                line_number: line_numbers.get(i).copied().unwrap_or(0),
            }
        })
        .collect()
}

fn make_pattern(raw_path: &str) -> Pattern {
    lazy_static! {
        static ref ESCAPE_REGEX: Regex = Regex::new(r"\\(\[|\])").unwrap();
    }
    // Replaces "\["=>"[[]", "\]"=>"[]]". Ideally we should escape space also, but I hope no one will use spaces in paths.
    let path = ESCAPE_REGEX.replace_all(raw_path, "[$1]").into_owned();
    // if pattern starts with anchor or explicit wild card, it should
    // match any prefix
    let prefixed = if path.starts_with('*') || path.starts_with('/') {
        path.to_owned()
    } else {
        format!("**/{}", path)
    };
    // if pattern starts with anchor it should only match paths
    // relative to root
    let mut normalized = prefixed.trim_start_matches('/').to_string();
    // if pattern ends with /, it should match children of that directory
    if normalized.ends_with('/') {
        normalized.push_str("**");
    }
    Pattern::new(&normalized).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    const EXAMPLE: &str = r"# This is a comment.
# Each line is a file pattern followed by one or more owners.

# These owners will be the default owners for everything in
# the repo. Unless a later match takes precedence,
# @global-owner1 and @global-owner2 will be requested for
# review when someone opens a pull request.
*       @global-owner1 @global-owner2

# Order is important; the last matching pattern takes the most
# precedence. When someone opens a pull request that only
# modifies JS files, only @js-owner and not the global
# owner(s) will be requested for a review.
*.js    @js-owner

# You can also use email addresses if you prefer. They'll be
# used to look up users just like we do for commit author
# emails.
*.go docs@example.com

# In this example, @doctocat owns any files in the build/logs
# directory at the root of the repository and any of its
# subdirectories.
/build/logs/ @doctocat

# The `docs/*` pattern will match files like
# `docs/getting-started.md` but not further nested files like
# `docs/build-app/troubleshooting.md`.
docs/*  docs@example.com

# In this example, @octocat owns any file in an apps directory
# anywhere in your repository.
apps/ @octocat

# In this example, @doctocat owns any file in the `/docs`
# directory in the root of your repository.
/docs/ @doctocat
";

    #[test]
    fn owner_parses() {
        assert!("@user".parse() == Ok(Owner::Username("@user".into())));
        assert!("@org/team".parse() == Ok(Owner::Team("@org/team".into())));
        assert!("user@domain.com".parse() == Ok(Owner::Email("user@domain.com".into())));
        assert!("bogus".parse::<Owner>() == Err("not an owner".into()));
    }

    #[test]
    fn owner_displays() {
        assert!(Owner::Username("@user".into()).to_string() == "@user");
        assert!(Owner::Team("@org/team".into()).to_string() == "@org/team");
        assert!(Owner::Email("user@domain.com".into()).to_string() == "user@domain.com");
    }

    #[test]
    fn from_reader_parses() {
        let owners = from_reader(EXAMPLE.as_bytes());
        let expected_paths = vec![
            (Pattern::new("docs/**").unwrap(), vec![Owner::Username("@doctocat".into())], None),
            (Pattern::new("**/apps/**").unwrap(), vec![Owner::Username("@octocat".into())], None),
            (Pattern::new("**/docs/*").unwrap(), vec![Owner::Email("docs@example.com".into())], None),
            (Pattern::new("build/logs/**").unwrap(), vec![Owner::Username("@doctocat".into())], None),
            (Pattern::new("*.go").unwrap(), vec![Owner::Email("docs@example.com".into())], None),
            (Pattern::new("*.js").unwrap(), vec![Owner::Username("@js-owner".into())], None),
            (
                Pattern::new("*").unwrap(),
                vec![
                    Owner::Username("@global-owner1".into()),
                    Owner::Username("@global-owner2".into()),
                ],
                None,
            ),
        ];
        let expected_meta = build_meta(&expected_paths, &[]);
        assert_eq!(
            owners,
            Owners {
                paths: expected_paths,
                meta: expected_meta,
            }
        )
    }

    #[test]
    fn owners_owns_wildcard() {
        let owners = from_reader(EXAMPLE.as_bytes());
        assert_eq!(
            owners.of("foo.txt"),
            Some(&vec![
                Owner::Username("@global-owner1".into()),
                Owner::Username("@global-owner2".into()),
            ])
        );
        assert_eq!(
            owners.of("foo/bar.txt"),
            Some(&vec![
                Owner::Username("@global-owner1".into()),
                Owner::Username("@global-owner2".into()),
            ])
        )
    }

    #[test]
    fn owners_owns_js_extention() {
        let owners = from_reader(EXAMPLE.as_bytes());
        assert_eq!(
            owners.of("foo.js"),
            Some(&vec![Owner::Username("@js-owner".into())])
        );
        assert_eq!(
            owners.of("foo/bar.js"),
            Some(&vec![Owner::Username("@js-owner".into())])
        )
    }

    #[test]
    fn owners_owns_go_extention() {
        let owners = from_reader(EXAMPLE.as_bytes());
        assert_eq!(
            owners.of("foo.go"),
            Some(&vec![Owner::Email("docs@example.com".into())])
        );
        assert_eq!(
            owners.of("foo/bar.go"),
            Some(&vec![Owner::Email("docs@example.com".into())])
        )
    }

    #[test]
    fn owners_owns_anchored_build_logs() {
        let owners = from_reader(EXAMPLE.as_bytes());
        // relative to root
        assert_eq!(
            owners.of("build/logs/foo.go"),
            Some(&vec![Owner::Username("@doctocat".into())])
        );
        assert_eq!(
            owners.of("build/logs/foo/bar.go"),
            Some(&vec![Owner::Username("@doctocat".into())])
        );
        // not relative to root
        assert_eq!(
            owners.of("foo/build/logs/foo.go"),
            Some(&vec![Owner::Email("docs@example.com".into())])
        )
    }

    #[test]
    fn owners_owns_unanchored_docs() {
        let owners = from_reader(EXAMPLE.as_bytes());
        // docs anywhere
        assert_eq!(
            owners.of("foo/docs/foo.js"),
            Some(&vec![Owner::Email("docs@example.com".into())])
        );
        assert_eq!(
            owners.of("foo/bar/docs/foo.js"),
            Some(&vec![Owner::Email("docs@example.com".into())])
        );
        // but not nested
        assert_eq!(
            owners.of("foo/bar/docs/foo/foo.js"),
            Some(&vec![Owner::Username("@js-owner".into())])
        )
    }

    #[test]
    fn owners_owns_unanchored_apps() {
        let owners = from_reader(EXAMPLE.as_bytes());
        assert_eq!(
            owners.of("foo/apps/foo.js"),
            Some(&vec![Owner::Username("@octocat".into())])
        )
    }

    #[test]
    fn owners_owns_anchored_docs() {
        let owners = from_reader(EXAMPLE.as_bytes());
        // relative to root
        assert_eq!(
            owners.of("docs/foo.js"),
            Some(&vec![Owner::Username("@doctocat".into())])
        )
    }

    #[test]
    fn implied_children_owners() {
        let owners = from_reader("foo/bar @doug".as_bytes());
        assert_eq!(
            owners.of("foo/bar/baz.rs"),
            Some(&vec![Owner::Username("@doug".into())])
        )
    }

    /// Regression coverage for CODEOWNERS rules whose *paths* contain
    /// bracket characters (e.g. Next.js dynamic route segments like
    /// `[groupId]`). `make_pattern` escapes `\[` → `[[]` and `\]` → `[]]`
    /// so the resulting glob character-classes match a literal `[`/`]`.
    /// This test locks in both the pattern rewrite and the ownership
    /// resolution — both through the canonical `Owners::of` slow path
    /// and the batch fast path used for whole-repo resolution.
    #[test]
    fn owners_owns_paths_with_brackets() {
        let raw = "/client/apps/dashboard/pages/destinations/\\[groupId\\]/*.tsx @dash-owners\n";
        let owners = from_reader(raw.as_bytes());

        // Slow path: literal match on a file inside `[groupId]`.
        let file = "client/apps/dashboard/pages/destinations/[groupId]/data-security.page.tsx";
        assert_eq!(
            owners.of(file),
            Some(&vec![Owner::Username("@dash-owners".into())]),
            "Owners::of should match a file under a bracketed dynamic route segment",
        );

        // Slow path: a non-matching path with different bracket contents
        // should NOT be owned by the bracketed rule — that would mean our
        // character class silently swallowed the inner text.
        let miss = "client/apps/dashboard/pages/destinations/regular/data-security.page.tsx";
        assert_eq!(
            owners.of(miss),
            None,
            "the bracketed rule must only match literal `[groupId]`, not any segment",
        );

        // Fast path parity: the batch resolver used for the whole-repo
        // sweep must produce the same owner index. We reconstruct the
        // ancestor cache the same way `main.rs` does for a single file.
        let owner_strings = owners.owner_strings();
        let path = Path::new(file);
        let ancestor_index = {
            let mut acc: Option<usize> = None;
            let mut p = path.parent();
            while let Some(dir) = p {
                if let Some(i) = owners.direct_match_index_at(dir) {
                    acc = Some(match acc {
                        Some(prev) => prev.min(i),
                        None => i,
                    });
                }
                p = dir.parent();
            }
            acc
        };
        let fast_string = owners
            .of_index_with_ancestor(path, ancestor_index)
            .map(|i| owner_strings[i].clone());
        assert_eq!(fast_string.as_deref(), Some("@dash-owners"));
    }

    /// The `/*` rule form is documented as "direct children only" in
    /// GitHub's CODEOWNERS spec, and the matcher preserves that. Verify
    /// it still holds when the path segment contains bracket characters
    /// (e.g. `[groupId]/child.tsx` should still match `[groupId]/*` but
    /// `[groupId]/nested/child.tsx` should not).
    #[test]
    fn owners_owns_bracketed_direct_children_only() {
        let raw = "/pages/\\[id\\]/* @dyn-owners\n";
        let owners = from_reader(raw.as_bytes());

        assert_eq!(
            owners.of("pages/[id]/index.tsx"),
            Some(&vec![Owner::Username("@dyn-owners".into())]),
            "direct child of bracketed dir should match `/*` rule",
        );
        assert_eq!(
            owners.of("pages/[id]/nested/deep.tsx"),
            None,
            "nested descendant should NOT match a `/*` rule",
        );
    }

    #[test]
    fn make_pattern_escapes() {
        let pattern = make_pattern(
            r"/client/apps/dashboard/pages/dashboard/destinations/\[groupId\]/data-security.page.tsx",
        );

        assert_eq!(
            pattern.to_string(),
            r"client/apps/dashboard/pages/dashboard/destinations/[[]groupId[]]/data-security.page.tsx"
        )
    }

    /// `of_with_comment` must be behaviorally identical to calling `of`
    /// and `comment_of` separately. Guards the single-scan optimization
    /// used by `get_changed_codeowners_for_branch`.
    #[test]
    fn of_with_comment_matches_separate_calls() {
        let raw = r"* @global-owner
docs/* @docs-team #!required
*.js @js-owner
";
        let owners = from_reader(raw.as_bytes());
        for case in &[
            "foo.txt",
            "foo.js",
            "docs/getting-started.md",
            "nope",
        ] {
            assert_eq!(
                owners.of_with_comment(case),
                (owners.of(case), owners.comment_of(case)),
                "combined lookup diverged from separate lookups for `{case}`",
            );
        }
    }
}
