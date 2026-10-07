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

use std::collections::BTreeMap;
#[allow(unused_imports)]
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

/// User response to an overwrite prompt.
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub(super) enum OverwriteChoice {
    Yes,
    No,
    OverwriteAll,
    SkipAll,
    Abort,
}

/// Prompts the user for overwrite confirmation on stderr, reads choice from stdin.
///
/// Returns `None` if stdin is not a terminal (non-interactive).
#[allow(clippy::missing_const_for_fn)]
pub(super) fn prompt_overwrite(path: &Path) -> Option<OverwriteChoice> {
    #[cfg(test)]
    {
        let _ = path;
        None
    }
    #[cfg(not(test))]
    {
        if !io::stdin().is_terminal() {
            return None;
        }

        loop {
            eprint!(
                "File `{}` already exists. Overwrite? [y]es / [n]o / [o]verwrite all / [s]kip all / [a]bort: ",
                path.display()
            );
            let _ = io::stderr().flush();

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => return Some(OverwriteChoice::Abort),
                Err(_) => return None,
                Ok(_) => {}
            }

            match input.trim().to_lowercase().as_str() {
                "y" | "yes" => return Some(OverwriteChoice::Yes),
                "n" | "no" => return Some(OverwriteChoice::No),
                "o" | "overwrite" | "overwrite-all" | "overwriteall" | "all" => {
                    return Some(OverwriteChoice::OverwriteAll);
                }
                "s" | "skip" | "skip-all" | "skipall" => return Some(OverwriteChoice::SkipAll),
                "a" | "abort" => return Some(OverwriteChoice::Abort),
                _ => {
                    eprintln!("Please answer y/n/o/s/a.");
                }
            }
        }
    }
}

use super::RouterError;

/// Unified overwrite decision combining the force/skip-all guard and interactive prompt.
///
/// Returns `Ok(true)` if the caller should write the file, `Ok(false)` if it should skip.
/// Returns `Err(RouterError::NonInteractiveOverwrite)` if file exists in non-TTY mode without force.
pub fn resolve_overwrite(
    path: &Path,
    force: bool,
    skip_all: &mut bool,
    overwrite_all: &mut bool,
    skipped: &mut Vec<String>,
) -> Result<bool, RouterError> {
    if force || *overwrite_all || !path.exists() {
        return Ok(true);
    }
    if *skip_all {
        skipped.push(path.to_string_lossy().to_string());
        return Ok(false);
    }
    match prompt_overwrite(path) {
        Some(OverwriteChoice::Yes) => Ok(true),
        Some(OverwriteChoice::OverwriteAll) => {
            *overwrite_all = true;
            Ok(true)
        }
        Some(OverwriteChoice::No) => {
            skipped.push(path.to_string_lossy().to_string());
            Ok(false)
        }
        Some(OverwriteChoice::SkipAll) => {
            *skip_all = true;
            skipped.push(path.to_string_lossy().to_string());
            Ok(false)
        }
        Some(OverwriteChoice::Abort) => Err(RouterError::Aborted),
        None => Err(RouterError::NonInteractiveOverwrite {
            path: path.to_string_lossy().to_string(),
        }),
    }
}

/// How to treat one file while several harnesses write during a single command.
#[derive(Debug, PartialEq, Eq)]
pub enum SharedWrite {
    /// The caller should write `content` and then [`WriteSession::record`] it.
    Write,
    /// This command already wrote identical bytes to the path.
    Deduped,
    /// The user declined to replace a file that already existed.
    Skipped,
}

/// Bytes already written by the current install or update command.
///
/// A later harness that targets one of these paths with the same bytes is not an
/// overwrite. Different bytes are a collision. Files that existed before the
/// command are not recorded here, so they still require `--force`.
#[derive(Debug, Default)]
pub struct WriteSession {
    written: BTreeMap<PathBuf, Vec<u8>>,
}

impl WriteSession {
    /// Decides whether `content` should be written to `path`.
    ///
    /// `yes` skips prompts and refuses files that were not created by this
    /// session. `--force` still replaces those pre-existing files.
    // reason: overwrite policy is force/yes/skip/overwrite-all plus the session;
    // packing them into another struct would hide the call-site flags.
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    pub fn resolve(
        &self,
        path: &Path,
        content: &[u8],
        force: bool,
        yes: bool,
        skip_all: &mut bool,
        overwrite_all: &mut bool,
        skipped: &mut Vec<String>,
        conflict_label: &str,
    ) -> Result<SharedWrite, RouterError> {
        if let Some(previous) = self.written.get(path) {
            if previous.as_slice() == content {
                return Ok(SharedWrite::Deduped);
            }
            return Err(RouterError::PathCollision {
                path: path.to_string_lossy().to_string(),
                colliding_skills: format!(
                    "{conflict_label} conflicts with an earlier write in this install"
                ),
            });
        }
        // `--yes` means "do not prompt", not "overwrite". A file that already
        // existed before this command is still refused unless `--force` is set.
        if yes && !force && path.exists() {
            return Err(RouterError::NonInteractiveOverwrite {
                path: path.to_string_lossy().to_string(),
            });
        }
        if resolve_overwrite(path, force, skip_all, overwrite_all, skipped)? {
            Ok(SharedWrite::Write)
        } else {
            Ok(SharedWrite::Skipped)
        }
    }

    /// Records bytes this command has successfully written.
    pub fn record(&mut self, path: PathBuf, content: Vec<u8>) {
        self.written.insert(path, content);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn force_and_nonexistent_files_overwrite_without_prompt() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("test.txt");

        let mut skip_all = false;
        let mut overwrite_all = false;
        let mut skipped = Vec::new();

        assert!(
            resolve_overwrite(
                &file,
                false,
                &mut skip_all,
                &mut overwrite_all,
                &mut skipped
            )
            .unwrap()
        );

        fs::write(&file, "content").unwrap();
        assert!(
            resolve_overwrite(&file, true, &mut skip_all, &mut overwrite_all, &mut skipped)
                .unwrap()
        );
    }

    #[test]
    fn overwrite_all_state_bypasses_future_prompts() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("test.txt");
        fs::write(&file, "content").unwrap();

        let mut skip_all = false;
        let mut overwrite_all = true;
        let mut skipped = Vec::new();

        assert!(
            resolve_overwrite(
                &file,
                false,
                &mut skip_all,
                &mut overwrite_all,
                &mut skipped
            )
            .unwrap()
        );
        assert!(skipped.is_empty());
    }

    #[test]
    fn skip_all_state_skips_future_files() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("test.txt");
        fs::write(&file, "content").unwrap();

        let mut skip_all = true;
        let mut overwrite_all = false;
        let mut skipped = Vec::new();

        assert!(
            !resolve_overwrite(
                &file,
                false,
                &mut skip_all,
                &mut overwrite_all,
                &mut skipped
            )
            .unwrap()
        );
        assert_eq!(skipped.len(), 1);
    }

    #[test]
    fn non_interactive_returns_error_for_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("test.txt");
        fs::write(&file, "content").unwrap();

        let mut skip_all = false;
        let mut overwrite_all = false;
        let mut skipped = Vec::new();

        let err = resolve_overwrite(
            &file,
            false,
            &mut skip_all,
            &mut overwrite_all,
            &mut skipped,
        )
        .unwrap_err();

        assert!(matches!(err, RouterError::NonInteractiveOverwrite { .. }));
    }

    #[test]
    fn session_dedups_identical_bytes_and_rejects_different_bytes() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("SKILL.md");
        let mut session = WriteSession::default();
        let mut skip_all = false;
        let mut overwrite_all = false;
        let mut skipped = Vec::new();

        let action = session
            .resolve(
                &file,
                b"same",
                false,
                true,
                &mut skip_all,
                &mut overwrite_all,
                &mut skipped,
                "demo → codex",
            )
            .unwrap();
        assert_eq!(action, SharedWrite::Write);
        fs::write(&file, b"same").unwrap();
        session.record(file.clone(), b"same".to_vec());

        let again = session
            .resolve(
                &file,
                b"same",
                false,
                true,
                &mut skip_all,
                &mut overwrite_all,
                &mut skipped,
                "demo → opencode",
            )
            .unwrap();
        assert_eq!(again, SharedWrite::Deduped);

        let err = session
            .resolve(
                &file,
                b"different",
                false,
                true,
                &mut skip_all,
                &mut overwrite_all,
                &mut skipped,
                "demo → factory",
            )
            .unwrap_err();
        assert!(matches!(err, RouterError::PathCollision { .. }));
        assert_eq!(fs::read(&file).unwrap(), b"same");
    }

    #[test]
    fn yes_refuses_file_that_existed_before_the_session() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("SKILL.md");
        fs::write(&file, b"user-owned").unwrap();
        let session = WriteSession::default();
        let mut skip_all = false;
        let mut overwrite_all = false;
        let mut skipped = Vec::new();

        let err = session
            .resolve(
                &file,
                b"incoming",
                false,
                true,
                &mut skip_all,
                &mut overwrite_all,
                &mut skipped,
                "demo → codex",
            )
            .unwrap_err();
        assert!(matches!(err, RouterError::NonInteractiveOverwrite { .. }));
        assert_eq!(fs::read(&file).unwrap(), b"user-owned");
    }
}
