// Copyright 2026 Oscar Yáñez Cisterna (@SkrOYC)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Source URL parser for `skillprism add`.
//!
//! Mirrors the v1 source forms supported by Vercel's `skills` CLI
//! (`src/source-parser.ts`), with focused support for:
//!
//! - local paths
//! - `github:` / `gitlab:` prefixes
//! - full GitHub / GitLab URLs (including tree URLs with ref + subpath)
//! - `owner/repo` shorthand, with optional subpath / skill filter / ref fragments
//! - `.well-known/agent-skills/index.json` discovery
//! - source aliases
//! - direct git URLs

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use miette::Diagnostic;
use thiserror::Error;

/// Errors that can occur while parsing a source string.
#[derive(Debug, Diagnostic, Error)]
pub enum SourceParseError {
    /// The source string is empty or contains only whitespace.
    #[error("source cannot be empty or whitespace")]
    EmptySource,

    /// The source is not a recognized v1 source form.
    #[error("unsupported source form: {input}")]
    #[diagnostic(help(
        "Supported forms: local path, github:owner/repo, gitlab:owner/repo, https://github.com/..., https://gitlab.com/..., owner/repo, owner/repo@skill, owner/repo#ref, owner/repo#ref@skill"
    ))]
    UnsupportedSource { input: String },

    /// The source contains an unsafe subpath with path-traversal segments.
    #[error("unsafe subpath in source: {subpath}")]
    #[diagnostic(help("Subpaths must not contain '..' segments."))]
    UnsafeSubpath { subpath: String },
}

/// A parsed source ready for fetching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedSource {
    /// GitHub repository source.
    GitHub {
        url: String,
        r#ref: Option<String>,
        subpath: Option<String>,
        skill_filter: Option<String>,
    },
    /// GitLab repository source (including self-hosted).
    GitLab {
        url: String,
        r#ref: Option<String>,
        subpath: Option<String>,
        skill_filter: Option<String>,
    },
    /// Generic git repository source.
    Git { url: String, r#ref: Option<String> },
    /// Local filesystem path.
    Local { path: PathBuf },
    /// Well-known skills index endpoint.
    WellKnown { url: String, index_path: String },
}

/// Returns the v1 source alias map.
pub fn source_aliases() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert(
        "coinbase/agentWallet".to_string(),
        "coinbase/agentic-wallet-skills".to_string(),
    );
    map
}

/// Parses a user-supplied source string into a structured source.
///
/// Whitespace-only input returns [`SourceParseError::EmptySource`].
pub fn parse_source(input: &str) -> Result<ParsedSource, SourceParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(SourceParseError::EmptySource);
    }

    // Local paths: absolute, explicit relative, or Windows drive letters.
    if is_local_path(input) {
        return Ok(ParsedSource::Local {
            path: normalize_local_path(input),
        });
    }

    let (input_without_fragment, fragment_ref, fragment_skill_filter) = parse_fragment_ref(input);
    let mut input = input_without_fragment;

    // Resolve source aliases before further parsing.
    if let Some(alias) = source_aliases().get(&input) {
        input.clone_from(alias);
    }

    // github:owner/repo -> owner/repo shorthand.
    if let Some(rest) = input.strip_prefix("github:") {
        let rebuilt = append_fragment_ref(
            rest,
            fragment_ref.as_deref(),
            fragment_skill_filter.as_deref(),
        );
        return parse_source(&rebuilt);
    }

    // gitlab:owner/repo -> https://gitlab.com/owner/repo.
    if let Some(rest) = input.strip_prefix("gitlab:") {
        let (repo_part, skill_filter) = split_at_skill_filter(rest);
        let rebuilt = format!("https://gitlab.com/{repo_part}");
        let mut parsed = parse_source(&rebuilt)?;
        if let ParsedSource::GitLab {
            skill_filter: ref mut filter,
            ..
        } = parsed
        {
            if skill_filter.is_some() {
                *filter = skill_filter.map(String::from);
            } else {
                filter.clone_from(&fragment_skill_filter);
            }
        }
        return Ok(parsed);
    }

    // Full GitHub / GitLab URLs.
    if let Some(parsed) = parse_url(&input, fragment_ref.clone(), fragment_skill_filter.clone()) {
        return Ok(parsed);
    }

    // GitHub shorthand with @skill filter: owner/repo@skill-name.
    if let Some(caps) = parse_owner_repo_at_skill(&input) {
        return Ok(ParsedSource::GitHub {
            url: format!("https://github.com/{}/{}.git", caps.owner, caps.repo),
            r#ref: fragment_ref,
            subpath: None,
            skill_filter: fragment_skill_filter.or(Some(caps.skill_filter)),
        });
    }

    // GitHub shorthand with optional subpath/skill filter: owner/repo[/subpath].
    if let Some(parts) = parse_shorthand(&input) {
        let subpath = parts.subpath.as_deref().map(sanitize_subpath).transpose()?;
        return Ok(ParsedSource::GitHub {
            url: format!("https://github.com/{}/{}.git", parts.owner, parts.repo),
            r#ref: fragment_ref,
            subpath,
            skill_filter: fragment_skill_filter,
        });
    }

    // Well-known URL: arbitrary HTTP(S) that is not a known git host/repo.
    if is_well_known_url(&input) {
        return Ok(ParsedSource::WellKnown {
            url: input,
            index_path: "/.well-known/agent-skills/index.json".to_string(),
        });
    }

    // Fallback: treat as a direct git URL.
    Ok(ParsedSource::Git {
        url: input,
        r#ref: fragment_ref,
    })
}

fn is_local_path(input: &str) -> bool {
    Path::new(input).is_absolute()
        || input.starts_with("./")
        || input.starts_with("../")
        || input == "."
        || input == ".."
        || input.starts_with('~')
        || is_windows_drive_path(input)
}

fn is_windows_drive_path(input: &str) -> bool {
    let bytes = input.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
}

fn normalize_local_path(input: &str) -> PathBuf {
    let expanded = input.strip_prefix('~').map_or_else(
        || input.to_string(),
        |rest| {
            std::env::var("HOME").map_or_else(
                |_| input.to_string(),
                |home| {
                    if rest.is_empty() {
                        home
                    } else {
                        format!("{home}{rest}")
                    }
                },
            )
        },
    );
    PathBuf::from(expanded)
}

fn parse_fragment_ref(input: &str) -> (String, Option<String>, Option<String>) {
    let Some(hash_pos) = input.find('#') else {
        return (input.to_string(), None, None);
    };
    let without = &input[..hash_pos];
    let fragment = &input[hash_pos + 1..];

    if fragment.is_empty() || !looks_like_git_source(without) {
        return (input.to_string(), None, None);
    }

    fragment.find('@').map_or_else(
        || (without.to_string(), Some(url_decode(fragment)), None),
        |at_pos| {
            let r#ref = Some(url_decode(&fragment[..at_pos]));
            let skill_filter = Some(url_decode(&fragment[at_pos + 1..]));
            (without.to_string(), r#ref, skill_filter)
        },
    )
}

fn looks_like_git_source(input: &str) -> bool {
    input.starts_with("github:")
        || input.starts_with("gitlab:")
        || input.starts_with("git@")
        || input.starts_with("ssh://")
        || input.starts_with("http://")
        || input.starts_with("https://")
        || is_shorthand(input)
}

fn is_shorthand(input: &str) -> bool {
    if input.contains(':') || input.starts_with('.') || input.starts_with('/') {
        return false;
    }
    let parts: Vec<&str> = input.split('/').collect();
    parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty()
}

fn append_fragment_ref(input: &str, r#ref: Option<&str>, skill_filter: Option<&str>) -> String {
    if r#ref.is_none() && skill_filter.is_none() {
        return input.to_string();
    }
    let mut out = input.to_string();
    if let Some(r) = r#ref {
        out.push('#');
        out.push_str(r);
    }
    if let Some(s) = skill_filter {
        out.push('@');
        out.push_str(s);
    }
    out
}

fn parse_url(
    input: &str,
    fragment_ref: Option<String>,
    fragment_skill_filter: Option<String>,
) -> Option<ParsedSource> {
    let (scheme, rest) = split_scheme(input)?;
    if scheme != "http" && scheme != "https" {
        return None;
    }

    let (hostname, path) = rest.split_once('/')?;
    let path = path.strip_suffix('/').unwrap_or(path);

    if hostname == "github.com" {
        parse_github_path(path, fragment_ref, fragment_skill_filter)
    } else if hostname == "gitlab.com" {
        parse_gitlab_path(
            path,
            fragment_ref,
            fragment_skill_filter,
            "https://gitlab.com",
        )
    } else if hostname.contains("gitlab") {
        parse_gitlab_path(
            path,
            fragment_ref,
            fragment_skill_filter,
            &format!("{scheme}://{hostname}"),
        )
    } else {
        None
    }
}

fn split_scheme(input: &str) -> Option<(&str, &str)> {
    input.split_once("://")
}

fn parse_github_path(
    path: &str,
    fragment_ref: Option<String>,
    fragment_skill_filter: Option<String>,
) -> Option<ParsedSource> {
    // /owner/repo/tree/ref/subpath
    if let Some((repo_part, tree_part)) = path.split_once("/tree/") {
        let parts: Vec<&str> = repo_part.split('/').collect();
        if parts.len() == 2 {
            let (owner, repo) = (parts[0], parts[1]);
            let repo = repo.strip_suffix(".git").unwrap_or(repo);
            let tree_parts: Vec<&str> = tree_part.splitn(2, '/').collect();
            let tree_ref = tree_parts[0].to_string();
            let subpath = if tree_parts.len() > 1 {
                Some(sanitize_subpath(tree_parts[1]).ok()?)
            } else {
                None
            };
            return Some(ParsedSource::GitHub {
                url: format!("https://github.com/{owner}/{repo}.git"),
                r#ref: Some(tree_ref),
                subpath,
                skill_filter: fragment_skill_filter,
            });
        }
        return None;
    }

    // /owner/repo
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() == 2 {
        let (owner, repo) = (parts[0], parts[1]);
        let repo = repo.strip_suffix(".git").unwrap_or(repo);
        return Some(ParsedSource::GitHub {
            url: format!("https://github.com/{owner}/{repo}.git"),
            r#ref: fragment_ref,
            subpath: None,
            skill_filter: fragment_skill_filter,
        });
    }

    None
}

fn parse_gitlab_path(
    path: &str,
    fragment_ref: Option<String>,
    fragment_skill_filter: Option<String>,
    base_url: &str,
) -> Option<ParsedSource> {
    // group/subgroup/repo/-/tree/ref/subpath
    if let Some((repo_part, tree_part)) = path.split_once("/-/tree/") {
        let repo_path = repo_part.strip_suffix(".git").unwrap_or(repo_part);
        let tree_parts: Vec<&str> = tree_part.splitn(2, '/').collect();
        let tree_ref = tree_parts[0].to_string();
        let subpath = if tree_parts.len() > 1 {
            Some(sanitize_subpath(tree_parts[1]).ok()?)
        } else {
            None
        };
        return Some(ParsedSource::GitLab {
            url: format!("{base_url}/{repo_path}.git"),
            r#ref: Some(tree_ref),
            subpath,
            skill_filter: fragment_skill_filter,
        });
    }

    // group/subgroup/repo
    let path = path.strip_suffix(".git").unwrap_or(path);
    if path.contains('/') {
        return Some(ParsedSource::GitLab {
            url: format!("{base_url}/{path}.git"),
            r#ref: fragment_ref,
            subpath: None,
            skill_filter: fragment_skill_filter,
        });
    }

    None
}

struct ShorthandParts {
    owner: String,
    repo: String,
    subpath: Option<String>,
}

fn parse_shorthand(input: &str) -> Option<ShorthandParts> {
    if !is_shorthand(input) {
        return None;
    }
    let parts: Vec<&str> = input.split('/').collect();
    if parts.len() < 2 {
        return None;
    }
    let owner = parts[0].to_string();
    let repo = parts[1].to_string();
    let subpath = if parts.len() > 2 {
        Some(parts[2..].join("/"))
    } else {
        None
    };
    Some(ShorthandParts {
        owner,
        repo,
        subpath,
    })
}

struct OwnerRepoAtSkill {
    owner: String,
    repo: String,
    skill_filter: String,
}

/// Splits a shorthand repo fragment such as `owner/repo@skill-name` into the
/// repo part and an optional skill-filter suffix.
fn split_at_skill_filter(input: &str) -> (&str, Option<&str>) {
    let Some(at_pos) = input.rfind('@') else {
        return (input, None);
    };
    let prefix = &input[..at_pos];
    let parts: Vec<&str> = prefix.split('/').collect();
    if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        (&input[..at_pos], Some(&input[at_pos + 1..]))
    } else {
        (input, None)
    }
}

fn parse_owner_repo_at_skill(input: &str) -> Option<OwnerRepoAtSkill> {
    if !is_shorthand(input) {
        return None;
    }
    let at_pos = input.rfind('@')?;
    let prefix = &input[..at_pos];
    let skill_filter = input[at_pos + 1..].to_string();

    let prefix_parts: Vec<&str> = prefix.split('/').collect();
    if prefix_parts.len() != 2 || prefix_parts[0].is_empty() || prefix_parts[1].is_empty() {
        return None;
    }

    Some(OwnerRepoAtSkill {
        owner: prefix_parts[0].to_string(),
        repo: prefix_parts[1].to_string(),
        skill_filter,
    })
}

fn is_well_known_url(input: &str) -> bool {
    let Some((scheme, rest)) = split_scheme(input) else {
        return false;
    };
    if scheme != "http" && scheme != "https" {
        return false;
    }

    let hostname = rest.split('/').next().unwrap_or(rest);
    let excluded = ["github.com", "gitlab.com", "raw.githubusercontent.com"];
    if excluded.contains(&hostname) {
        return false;
    }

    !std::path::Path::new(input)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("git"))
}

fn sanitize_subpath(subpath: &str) -> Result<String, SourceParseError> {
    let normalized = subpath.replace('\\', "/");
    if normalized.split('/').any(|s| s == "..") {
        return Err(SourceParseError::UnsafeSubpath {
            subpath: subpath.to_string(),
        });
    }
    Ok(normalized)
}

fn url_decode(value: &str) -> String {
    // Percent-decode the fragment value. For the simple values used here
    // (refs and skill names), a manual decoder is sufficient and avoids
    // adding a dependency.
    let mut out = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h1), Some(h2)) = (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2])) {
                out.push(char::from(h1 * 16 + h2));
                i += 3;
                continue;
            }
        }
        out.push(char::from(bytes[i]));
        i += 1;
    }
    out
}

const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_rejected() {
        assert!(matches!(
            parse_source("").unwrap_err(),
            SourceParseError::EmptySource
        ));
        assert!(matches!(
            parse_source("   ").unwrap_err(),
            SourceParseError::EmptySource
        ));
    }

    #[test]
    fn github_shorthand() {
        let ParsedSource::GitHub {
            url,
            r#ref,
            subpath,
            skill_filter,
        } = parse_source("anthropics/skills").unwrap()
        else {
            panic!("expected GitHub");
        };
        assert_eq!(url, "https://github.com/anthropics/skills.git");
        assert!(r#ref.is_none());
        assert!(subpath.is_none());
        assert!(skill_filter.is_none());
    }

    #[test]
    fn github_prefix() {
        let ParsedSource::GitHub { url, .. } = parse_source("github:anthropics/skills").unwrap()
        else {
            panic!("expected GitHub");
        };
        assert_eq!(url, "https://github.com/anthropics/skills.git");
    }

    #[test]
    fn gitlab_prefix() {
        let ParsedSource::GitLab { url, .. } = parse_source("gitlab:mygroup/myskill").unwrap()
        else {
            panic!("expected GitLab");
        };
        assert_eq!(url, "https://gitlab.com/mygroup/myskill.git");
    }

    #[test]
    fn gitlab_prefix_with_skill_filter() {
        let ParsedSource::GitLab {
            url, skill_filter, ..
        } = parse_source("gitlab:mygroup/myskill@pdf").unwrap()
        else {
            panic!("expected GitLab");
        };
        assert_eq!(url, "https://gitlab.com/mygroup/myskill.git");
        assert_eq!(skill_filter.as_deref(), Some("pdf"));
    }

    #[test]
    fn github_tree_url_with_subpath() {
        let ParsedSource::GitHub {
            url,
            r#ref,
            subpath,
            ..
        } = parse_source("https://github.com/owner/repo/tree/main/skills/pdf").unwrap()
        else {
            panic!("expected GitHub");
        };
        assert_eq!(url, "https://github.com/owner/repo.git");
        assert_eq!(r#ref.as_deref(), Some("main"));
        assert_eq!(subpath.as_deref(), Some("skills/pdf"));
    }

    #[test]
    fn github_tree_url_with_ref_only() {
        let ParsedSource::GitHub {
            url,
            r#ref,
            subpath,
            ..
        } = parse_source("https://github.com/owner/repo/tree/v1.2.3").unwrap()
        else {
            panic!("expected GitHub");
        };
        assert_eq!(url, "https://github.com/owner/repo.git");
        assert_eq!(r#ref.as_deref(), Some("v1.2.3"));
        assert!(subpath.is_none());
    }

    #[test]
    fn self_hosted_gitlab() {
        let ParsedSource::GitLab { url, .. } =
            parse_source("https://gitlab.example.com/team/project").unwrap()
        else {
            panic!("expected GitLab");
        };
        assert_eq!(url, "https://gitlab.example.com/team/project.git");
    }

    #[test]
    fn gitlab_tree_with_subpath() {
        let ParsedSource::GitLab {
            url,
            r#ref,
            subpath,
            ..
        } = parse_source("https://gitlab.com/group/subgroup/repo/-/tree/main/skills/pdf").unwrap()
        else {
            panic!("expected GitLab");
        };
        assert_eq!(url, "https://gitlab.com/group/subgroup/repo.git");
        assert_eq!(r#ref.as_deref(), Some("main"));
        assert_eq!(subpath.as_deref(), Some("skills/pdf"));
    }

    #[test]
    fn fragment_ref() {
        let ParsedSource::GitHub { url, r#ref, .. } = parse_source("owner/repo#v1.2.3").unwrap()
        else {
            panic!("expected GitHub");
        };
        assert_eq!(url, "https://github.com/owner/repo.git");
        assert_eq!(r#ref.as_deref(), Some("v1.2.3"));
    }

    #[test]
    fn fragment_ref_and_skill_filter() {
        let ParsedSource::GitHub {
            url,
            r#ref,
            skill_filter,
            ..
        } = parse_source("owner/repo#main@pdf").unwrap()
        else {
            panic!("expected GitHub");
        };
        assert_eq!(url, "https://github.com/owner/repo.git");
        assert_eq!(r#ref.as_deref(), Some("main"));
        assert_eq!(skill_filter.as_deref(), Some("pdf"));
    }

    #[test]
    fn shorthand_skill_filter() {
        let ParsedSource::GitHub {
            url, skill_filter, ..
        } = parse_source("owner/repo@pdf").unwrap()
        else {
            panic!("expected GitHub");
        };
        assert_eq!(url, "https://github.com/owner/repo.git");
        assert_eq!(skill_filter.as_deref(), Some("pdf"));
    }

    #[test]
    fn shorthand_subpath() {
        let ParsedSource::GitHub { url, subpath, .. } =
            parse_source("owner/repo/skills/pdf").unwrap()
        else {
            panic!("expected GitHub");
        };
        assert_eq!(url, "https://github.com/owner/repo.git");
        assert_eq!(subpath.as_deref(), Some("skills/pdf"));
    }

    #[test]
    fn alias_resolved() {
        let ParsedSource::GitHub { url, .. } = parse_source("coinbase/agentWallet").unwrap() else {
            panic!("expected GitHub");
        };
        assert_eq!(url, "https://github.com/coinbase/agentic-wallet-skills.git");
    }

    #[test]
    fn unknown_alias_falls_through() {
        let ParsedSource::GitHub { url, .. } = parse_source("unknown/repo").unwrap() else {
            panic!("expected GitHub");
        };
        assert_eq!(url, "https://github.com/unknown/repo.git");
    }

    #[test]
    fn well_known_url() {
        let ParsedSource::WellKnown { url, index_path } =
            parse_source("https://example.com").unwrap()
        else {
            panic!("expected WellKnown");
        };
        assert_eq!(url, "https://example.com");
        assert_eq!(index_path, "/.well-known/agent-skills/index.json");
    }

    #[test]
    fn direct_git_url() {
        let ParsedSource::Git { url, .. } =
            parse_source("https://git.example.com/team/project.git").unwrap()
        else {
            panic!("expected Git");
        };
        assert_eq!(url, "https://git.example.com/team/project.git");
    }

    #[test]
    fn ssh_git_url() {
        let ParsedSource::Git { url, .. } = parse_source("git@github.com:owner/repo.git").unwrap()
        else {
            panic!("expected Git");
        };
        assert_eq!(url, "git@github.com:owner/repo.git");
    }

    #[test]
    fn local_absolute_path() {
        let ParsedSource::Local { path } = parse_source("/tmp/my-skills").unwrap() else {
            panic!("expected Local");
        };
        assert_eq!(path, PathBuf::from("/tmp/my-skills"));
    }

    #[test]
    fn local_relative_path() {
        let ParsedSource::Local { path } = parse_source("./my-skills").unwrap() else {
            panic!("expected Local");
        };
        assert_eq!(path, PathBuf::from("./my-skills"));
    }

    #[test]
    fn local_windows_path() {
        let ParsedSource::Local { path } = parse_source("C:\\skills").unwrap() else {
            panic!("expected Local");
        };
        assert_eq!(path, PathBuf::from("C:\\skills"));
    }

    #[test]
    fn unsafe_subpath_rejected() {
        assert!(matches!(
            parse_source("owner/repo/../escape").unwrap_err(),
            SourceParseError::UnsafeSubpath { .. }
        ));
    }
}
