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

//! Network fetch layer for the distribution CLI.
//!
//! Shells out to `git` directly for shallow clones, matching the methodology
//! Vercel's `skills` CLI uses in `src/git.ts`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use miette::Diagnostic;
use thiserror::Error;

use super::source::mask_credentials;

/// Default clone timeout: 5 minutes.
const DEFAULT_CLONE_TIMEOUT_MS: u64 = 300_000;

/// Default timeout for the `gh auth status` probe: 5 seconds.
const GH_AUTH_PROBE_TIMEOUT_MS: u64 = 5_000;

/// Environment variable name for overriding the clone timeout.
const CLONE_TIMEOUT_ENV: &str = "SKILLPRISM_CLONE_TIMEOUT_MS";

/// Errors that can occur while fetching a remote source.
#[derive(Debug, Diagnostic, Error)]
pub enum NetworkError {
    /// A required external command is not installed or not on PATH.
    #[error("`{command}` is required but was not found on PATH")]
    #[diagnostic(help("Install {command} and ensure it is on your PATH."))]
    CommandNotFound { command: String },

    /// The clone operation timed out.
    #[error("Clone timed out after {seconds}s")]
    #[diagnostic(help(
        "Raise the timeout with SKILLPRISM_CLONE_TIMEOUT_MS=600000 (10m), or clone manually and pass the local path to 'skillprism add'."
    ))]
    CloneTimeout { seconds: u64 },

    /// Authentication failed after exhausting all fallbacks.
    #[error("Authentication failed for {url}")]
    #[diagnostic(help("{advice}"))]
    AuthFailure { url: String, advice: String },

    /// A generic fetch failure occurred.
    #[error("Failed to fetch {url}: {detail}")]
    #[diagnostic(help("Check the URL, network connectivity, and git credentials."))]
    FetchFailure { url: String, detail: String },

    /// An I/O error occurred while managing the temporary directory.
    #[error("I/O error: {0}")]
    #[diagnostic(help(
        "Check permissions and available space in the system temp directory used for clones."
    ))]
    Io(#[from] std::io::Error),
}

/// RAII guard that removes a temporary git clone directory on drop unless it
/// is explicitly kept.
struct CloneGuard(Option<PathBuf>);

impl CloneGuard {
    fn path(&self) -> &Path {
        self.0.as_ref().expect("guard holds a directory")
    }

    fn keep(mut self) -> PathBuf {
        self.0.take().expect("guard holds a directory")
    }
}

impl Drop for CloneGuard {
    fn drop(&mut self) {
        if let Some(dir) = self.0.take() {
            let _ = cleanup_temp_dir(&dir);
        }
    }
}

/// Information about a GitHub repository extracted from a URL.
struct GitHubRepoInfo {
    slug: String,
    ssh_url: String,
}

/// Fetches a git repository into a newly-created temporary directory and
/// returns the path to that directory.
///
/// For GitHub HTTPS URLs, this implements the three-layer auth chain:
/// 1. `git clone` with credential helper / SSH agent resolution.
/// 2. `gh repo clone` if `gh` is authenticated.
/// 3. SSH fallback with `BatchMode=yes`.
///
/// The caller is responsible for cleaning up the returned directory via
/// [`cleanup_temp_dir`].
pub fn fetch_git_repo(url: &str, r#ref: Option<&str>) -> Result<PathBuf, NetworkError> {
    let is_sha = r#ref.is_some_and(is_sha_ref);
    // SHA refs cannot be cloned with `--branch`; fetch the default branch first
    // and then checkout the specific object.
    let clone_ref = if is_sha { None } else { r#ref };
    // Guard the temp dir immediately so it is removed on every error path
    // (ADR-008: "cleaned up on error or after use"). On success,
    // `maybe_checkout_sha` calls `keep()` to hand the directory to the caller.
    let guard = CloneGuard(Some(create_temp_dir()?));
    let args = clone_args(url, clone_ref, guard.path());

    if let Err(e) = run_git(&args) {
        let error_message = e.to_string();
        let is_timeout = is_timeout_message(&error_message);
        let is_auth = is_auth_failure(&error_message);
        let repo = parse_github_repo_url(url);
        let is_github_https = is_github_https_clone_url(url);

        if is_timeout {
            return Err(NetworkError::CloneTimeout {
                seconds: clone_timeout().as_secs(),
            });
        }

        if is_auth && is_github_https {
            if let Some(ref repo) = repo {
                // Layer 2: gh repo clone.
                let gh_result = try_gh_clone(repo, clone_ref);
                let gh_missing = matches!(
                    gh_result,
                    Err(NetworkError::CommandNotFound { command: ref c }) if c == "gh"
                );
                if let Ok(gh_dir) = gh_result {
                    return maybe_checkout_sha(CloneGuard(Some(gh_dir)), r#ref);
                }

                // Layer 3: SSH fallback.
                let ssh_dir = create_temp_dir()?;
                let ssh_guard = CloneGuard(Some(ssh_dir));
                let ssh_args = clone_args(&repo.ssh_url, clone_ref, ssh_guard.path());
                if run_git_with_ssh(&ssh_args, "ssh -o BatchMode=yes").is_ok() {
                    return maybe_checkout_sha(ssh_guard, r#ref);
                }

                if gh_missing {
                    return Err(NetworkError::CommandNotFound {
                        command: "gh".to_string(),
                    });
                }
            }

            let masked_url = mask_credentials(url);
            return Err(NetworkError::AuthFailure {
                advice: build_github_auth_error(&masked_url, repo.as_ref(), &error_message),
                url: masked_url,
            });
        }

        return Err(NetworkError::FetchFailure {
            url: mask_credentials(url),
            detail: error_message,
        });
    }

    maybe_checkout_sha(guard, r#ref)
}

fn maybe_checkout_sha(guard: CloneGuard, r#ref: Option<&str>) -> Result<PathBuf, NetworkError> {
    if let Some(sha) = r#ref.filter(|r| is_sha_ref(r)) {
        run_git_in_dir(
            guard.path(),
            &[
                "fetch".to_string(),
                "--depth".to_string(),
                "1".to_string(),
                "origin".to_string(),
                sha.to_string(),
            ],
        )?;
        run_git_in_dir(guard.path(), &["checkout".to_string(), sha.to_string()])?;
    }
    Ok(guard.keep())
}

/// Queries the remote HEAD of a ref without cloning.
///
/// Returns the current SHA-1 the remote resolves the ref to, or `None` if the
/// ref is not advertised. This is used by `update` to short-circuit the no-op
/// case.
pub fn git_remote_head(url: &str, r#ref: &str) -> Result<Option<String>, NetworkError> {
    let output = run_git_output(&["ls-remote".to_string(), url.to_string(), r#ref.to_string()])?;

    // Known tradeoff: we take the first `ls-remote` line. For an *annotated* tag
    // that is the tag-object SHA (`refs/tags/x`), not the commit it peels to
    // (`refs/tags/x^{}`), whereas the stored `resolved_ref` is the commit SHA.
    // So the cheap no-op short-circuit in `update` misses for annotated tags and
    // falls through to a full clone — which still renders the same bytes and
    // reports "up to date", so the end state is correct, only slower. Branches
    // and lightweight tags resolve straight to the commit and hit the fast path.
    for line in output.lines() {
        let mut parts = line.split_whitespace();
        if let Some(sha) = parts.next() {
            return Ok(Some(sha.to_string()));
        }
    }
    Ok(None)
}

/// Reads the current `HEAD` SHA of a local git repository.
pub fn git_dir_head(dir: &Path) -> Result<String, NetworkError> {
    let output = run_git_output(&[
        "-C".to_string(),
        dir.to_string_lossy().to_string(),
        "rev-parse".to_string(),
        "HEAD".to_string(),
    ])?;
    Ok(output.trim().to_string())
}

/// Resolves the default branch name of a remote git repository.
///
/// Returns `Ok(Some(branch))` when the remote advertises a `HEAD` ref,
/// `Ok(None)` when it does not, and `Err` on network/git failure.
pub fn git_default_branch(url: &str) -> Result<Option<String>, NetworkError> {
    let output = run_git_output(&[
        "ls-remote".to_string(),
        "--symref".to_string(),
        url.to_string(),
        "HEAD".to_string(),
    ])?;

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("ref: refs/heads/") {
            let branch = rest.split('\t').next().unwrap_or(rest);
            return Ok(Some(branch.to_string()));
        }
    }
    Ok(None)
}

/// Removes a temporary directory, validating that it lives under the system
/// temp directory first.
pub fn cleanup_temp_dir(dir: &Path) -> Result<(), NetworkError> {
    let normalized_dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let tmp = std::env::temp_dir();
    let normalized_tmp = std::fs::canonicalize(&tmp).unwrap_or(tmp);

    if !normalized_dir.starts_with(&normalized_tmp) {
        return Err(NetworkError::FetchFailure {
            url: dir.to_string_lossy().to_string(),
            detail: "refusing to clean up directory outside of temp dir".to_string(),
        });
    }

    std::fs::remove_dir_all(dir)?;
    Ok(())
}

fn create_temp_dir() -> Result<PathBuf, NetworkError> {
    let base = std::env::temp_dir();
    let unique = format!(
        "skillprism-clone-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        fast_random()
    );
    let dir = base.join(unique);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn fast_random() -> u32 {
    // A simple xorshift-based random number using the current nanosecond time
    // as seed. Uniqueness for temp dirs is provided by the pid + timestamp; the
    // random suffix is defense-in-depth against collisions inside the same
    // nanosecond.
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let mut x = seed.max(1);
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    x
}

fn clone_args(url: &str, r#ref: Option<&str>, dest: &Path) -> Vec<String> {
    let mut args = vec![
        "clone".to_string(),
        "--depth".to_string(),
        "1".to_string(),
        "--single-branch".to_string(),
    ];
    if let Some(r) = r#ref {
        args.push("--branch".to_string());
        args.push(r.to_string());
    }
    args.push(url.to_string());
    args.push(dest.to_string_lossy().to_string());
    args
}

/// Returns true when `r#ref` looks like a Git SHA object id.
///
/// Accepts full 40-character SHAs as well as short SHAs (7-39 characters) that
/// are composed entirely of hexadecimal digits. Branch or tag names that happen
/// to match this pattern will be treated as SHAs.
pub fn is_sha_ref(r#ref: &str) -> bool {
    (7..=40).contains(&r#ref.len()) && r#ref.chars().all(|c| c.is_ascii_hexdigit())
}

fn clone_timeout() -> Duration {
    std::env::var(CLONE_TIMEOUT_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|&ms| ms > 0)
        .map_or_else(
            || Duration::from_millis(DEFAULT_CLONE_TIMEOUT_MS),
            Duration::from_millis,
        )
}

fn git_base_env(cmd: &mut Command) {
    cmd.env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes")
        .env("GIT_LFS_SKIP_SMUDGE", "1")
        .env("GIT_CONFIG_COUNT", "4")
        .env("GIT_CONFIG_KEY_0", "filter.lfs.required")
        .env("GIT_CONFIG_VALUE_0", "false")
        .env("GIT_CONFIG_KEY_1", "filter.lfs.smudge")
        .env("GIT_CONFIG_VALUE_1", "")
        .env("GIT_CONFIG_KEY_2", "filter.lfs.clean")
        .env("GIT_CONFIG_VALUE_2", "")
        .env("GIT_CONFIG_KEY_3", "filter.lfs.process")
        .env("GIT_CONFIG_VALUE_3", "");
}

fn run_git(args: &[String]) -> Result<(), NetworkError> {
    run_git_with_dir(None, args)
}

fn run_git_in_dir(dir: &Path, args: &[String]) -> Result<(), NetworkError> {
    run_git_with_dir(Some(dir), args)
}

fn run_git_with_dir(dir: Option<&Path>, args: &[String]) -> Result<(), NetworkError> {
    let mut cmd = Command::new("git");
    git_base_env(&mut cmd);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let timeout = clone_timeout();
    let output = run_command_with_timeout("git", cmd, timeout)?;

    if output.status.success() {
        Ok(())
    } else {
        // Mask credentials: git's stderr can echo the remote URL, and a PAT in
        // the userinfo position (`https://TOKEN@host`) would otherwise leak.
        let detail = mask_credentials(&String::from_utf8_lossy(&output.stderr));
        if is_timeout_message(&detail) {
            Err(NetworkError::CloneTimeout {
                seconds: timeout.as_secs(),
            })
        } else {
            Err(NetworkError::FetchFailure {
                url: mask_credentials(&args.join(" ")),
                detail,
            })
        }
    }
}

fn run_git_with_ssh(args: &[String], ssh_command: &str) -> Result<(), NetworkError> {
    let mut cmd = Command::new("git");
    git_base_env(&mut cmd);
    cmd.env("GIT_SSH_COMMAND", ssh_command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let timeout = clone_timeout();
    let output = run_command_with_timeout("git", cmd, timeout)?;

    if output.status.success() {
        Ok(())
    } else {
        // Mask credentials: git's stderr can echo the remote URL, and a PAT in
        // the userinfo position (`https://TOKEN@host`) would otherwise leak.
        let detail = mask_credentials(&String::from_utf8_lossy(&output.stderr));
        if is_timeout_message(&detail) {
            Err(NetworkError::CloneTimeout {
                seconds: timeout.as_secs(),
            })
        } else {
            Err(NetworkError::FetchFailure {
                url: mask_credentials(&args.join(" ")),
                detail,
            })
        }
    }
}

fn run_git_output(args: &[String]) -> Result<String, NetworkError> {
    let mut cmd = Command::new("git");
    cmd.env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = run_command_with_timeout("git", cmd, Duration::from_secs(30))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        // Mask credentials: git's stderr can echo the remote URL, and a PAT in
        // the userinfo position (`https://TOKEN@host`) would otherwise leak.
        let detail = mask_credentials(&String::from_utf8_lossy(&output.stderr));
        Err(NetworkError::FetchFailure {
            url: mask_credentials(&args.join(" ")),
            detail,
        })
    }
}

fn run_command_with_timeout(
    command: &str,
    mut cmd: Command,
    timeout: Duration,
) -> Result<std::process::Output, NetworkError> {
    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            NetworkError::CommandNotFound {
                command: command.to_string(),
            }
        } else {
            NetworkError::Io(e)
        }
    })?;

    // Drain stdout/stderr on background threads so long-running commands
    // (e.g. large git clones) cannot deadlock when filling the OS pipe buffers.
    let stdout_handle = child.stdout.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = std::io::Read::read_to_end(&mut pipe, &mut buf);
            buf
        })
    });
    let stderr_handle = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = std::io::Read::read_to_end(&mut pipe, &mut buf);
            buf
        })
    });

    let start = std::time::Instant::now();
    let poll_interval = Duration::from_millis(50);

    let status = loop {
        if let Some(status) = child.try_wait().map_err(NetworkError::Io)? {
            break status;
        }

        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(NetworkError::CloneTimeout {
                seconds: timeout.as_secs(),
            });
        }

        std::thread::sleep(poll_interval);
    };

    let stdout = stdout_handle
        .and_then(|h| h.join().ok())
        .unwrap_or_default();
    let stderr = stderr_handle
        .and_then(|h| h.join().ok())
        .unwrap_or_default();

    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
}

fn is_timeout_message(message: &str) -> bool {
    let lower = message.to_lowercase();
    // Match git/curl's actual timeout phrasings ("Connection timed out",
    // "Operation timed out", curl's "Timeout was reached") rather than a bare
    // "timeout" substring, which would false-positive on repo names, branches,
    // or URLs surfaced in the error text (e.g. a repo literally named
    // "timeout"). The wall-clock timeout itself is enforced structurally in
    // `run_command_with_timeout`; this only classifies git's own error output.
    lower.contains("timed out") || lower.contains("timeout was reached")
}

fn is_auth_failure(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("authentication failed")
        || lower.contains("could not read username")
        || lower.contains("permission denied")
        || lower.contains("requested url returned error: 403")
        || is_github_sso_auth_error(&lower)
}

fn is_github_sso_auth_error(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("saml sso")
        || lower.contains("enforced sso")
        || lower.contains("enabled or enforced saml")
        || lower.contains("re-authorize the oauth application")
}

fn parse_github_repo_url(url: &str) -> Option<GitHubRepoInfo> {
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let path = rest.strip_suffix(".git").unwrap_or(rest);
        let (owner, repo) = path.split_once('/')?;
        return Some(GitHubRepoInfo {
            slug: format!("{owner}/{repo}"),
            ssh_url: format!("git@github.com:{owner}/{repo}.git"),
        });
    }

    let (_scheme, after_scheme) = url.split_once("://")?;
    let (host, path) = after_scheme.split_once('/').unwrap_or((after_scheme, ""));
    if !host.eq_ignore_ascii_case("github.com") {
        return None;
    }
    let path = path.split_once('?').map_or(path, |(p, _)| p);
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, repo) = path.split_once('/')?;
    let repo = repo.split_once('/').map_or(repo, |(r, _)| r);
    Some(GitHubRepoInfo {
        slug: format!("{owner}/{repo}"),
        ssh_url: format!("git@github.com:{owner}/{repo}.git"),
    })
}

fn is_github_https_clone_url(url: &str) -> bool {
    url.starts_with("https://github.com/")
}

fn try_gh_clone(repo: &GitHubRepoInfo, r#ref: Option<&str>) -> Result<PathBuf, NetworkError> {
    let mut clone_target = repo.slug.clone();

    let mut probe_cmd = Command::new("gh");
    probe_cmd
        .args(["auth", "status", "-h", "github.com"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let probe = run_command_with_timeout(
        "gh",
        probe_cmd,
        Duration::from_millis(GH_AUTH_PROBE_TIMEOUT_MS),
    )?;

    if !probe.status.success() {
        return Err(NetworkError::FetchFailure {
            url: repo.slug.clone(),
            detail: "gh not authenticated".to_string(),
        });
    }

    let status_output = format!(
        "{}{}",
        String::from_utf8_lossy(&probe.stdout),
        String::from_utf8_lossy(&probe.stderr)
    );
    if status_output
        .to_lowercase()
        .contains("git operations protocol:")
    {
        // If gh is using SSH protocol, prefer the SSH URL for the clone.
        clone_target.clone_from(&repo.ssh_url);
    }

    let temp_dir = create_temp_dir()?;
    let mut args = vec!["repo".to_string(), "clone".to_string(), clone_target];
    args.push(temp_dir.to_string_lossy().to_string());
    args.push("--".to_string());
    args.push("--depth=1".to_string());
    if let Some(r) = r#ref {
        args.push("--branch".to_string());
        args.push(r.to_string());
    }

    let mut clone_cmd = Command::new("gh");
    clone_cmd
        .args(&args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let status = run_command_with_timeout("gh", clone_cmd, clone_timeout())?;

    if status.status.success() {
        Ok(temp_dir)
    } else {
        let _ = cleanup_temp_dir(&temp_dir);
        Err(NetworkError::FetchFailure {
            url: repo.slug.clone(),
            detail: mask_credentials(&String::from_utf8_lossy(&status.stderr)),
        })
    }
}

fn build_github_auth_error(url: &str, repo: Option<&GitHubRepoInfo>, message: &str) -> String {
    if let Some(repo) = repo {
        if is_github_sso_auth_error(message) {
            return format!(
                "GitHub blocked HTTPS access to {url} because the organization enforces SAML SSO.\n\
                 skills tried your existing git credentials and available fallbacks, but none succeeded.\n\
                 - Re-authorize your GitHub credentials/app for that org's SSO policy\n\
                 - Or rerun with SSH: skillprism add {ssh}\n\
                 - Verify access with: gh auth status -h github.com or ssh -T git@github.com",
                ssh = repo.ssh_url
            );
        }

        return format!(
            "Authentication failed for {url}.\n\
             - For private repos, ensure you have access\n\
             - Retry with SSH: skillprism add {ssh}\n\
             - Check access with: gh auth status -h github.com or ssh -T git@github.com",
            ssh = repo.ssh_url
        );
    }

    format!(
        "Authentication failed for {url}.\n\
         - For private repos, ensure you have access\n\
         - For SSH: Check your keys with 'ssh -T git@github.com'\n\
         - For HTTPS: Run 'gh auth login' or configure git credentials"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn run_git(args: &[&str], cwd: &Path) {
        let status = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .expect("git should be installed");
        assert!(status.success(), "git {args:?} failed");
    }

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn git_default_branch_resolves_main() {
        let base_tmp = temp_dir();
        let base = base_tmp.path();
        let origin = base.join("origin.git");
        let work = base.join("work");
        fs::create_dir_all(&origin).unwrap();
        fs::create_dir_all(&work).unwrap();

        run_git(&["init", "--bare"], &origin);
        run_git(&["init"], &work);
        run_git(
            &["remote", "add", "origin", origin.to_str().unwrap()],
            &work,
        );

        fs::write(work.join("README.md"), "hello").unwrap();
        run_git(&["add", "README.md"], &work);
        run_git(
            &[
                "-c",
                "user.email=test@example.com",
                "-c",
                "user.name=Test",
                "commit",
                "-m",
                "init",
            ],
            &work,
        );
        run_git(&["push", "origin", "HEAD:main"], &work);
        run_git(&["symbolic-ref", "HEAD", "refs/heads/main"], &origin);

        let url = format!("file://{}", origin.canonicalize().unwrap().display());
        let branch = git_default_branch(&url).unwrap();
        assert_eq!(branch, Some("main".to_string()));
    }

    #[test]
    fn git_dir_head_returns_sha() {
        let base_tmp = temp_dir();
        let base = base_tmp.path();
        run_git(&["init"], base);
        fs::write(base.join("file.txt"), "content").unwrap();
        run_git(&["add", "file.txt"], base);
        run_git(
            &[
                "-c",
                "user.email=test@example.com",
                "-c",
                "user.name=Test",
                "commit",
                "-m",
                "init",
            ],
            base,
        );

        let head = git_dir_head(base).unwrap();
        assert_eq!(head.len(), 40);
        assert!(head.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
