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

#[allow(unused_imports)]
use std::io::{self, IsTerminal, Write};
use std::path::Path;

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
}
