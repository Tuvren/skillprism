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
                "File `{}` already exists. Overwrite? [y]es / [n]o / [s]kip all / [a]bort: ",
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
                "s" | "skip" | "skip-all" | "skipall" => return Some(OverwriteChoice::SkipAll),
                "a" | "abort" => return Some(OverwriteChoice::Abort),
                _ => {
                    eprintln!("Please answer y/n/s/a.");
                }
            }
        }
    }
}

/// Unified overwrite decision combining the force/skip-all guard and interactive prompt.
///
/// Returns `true` if the caller should write the file, `false` if it should skip.
/// Handles `skip_all` progression and abort exit internally.
pub fn resolve_overwrite(
    path: &Path,
    force: bool,
    skip_all: &mut bool,
    skipped: &mut Vec<String>,
) -> bool {
    if force || !path.exists() {
        return true;
    }
    if *skip_all {
        skipped.push(path.to_string_lossy().to_string());
        return false;
    }
    match prompt_overwrite(path) {
        Some(OverwriteChoice::Yes) => true,
        Some(OverwriteChoice::No) | None => {
            skipped.push(path.to_string_lossy().to_string());
            false
        }
        Some(OverwriteChoice::SkipAll) => {
            *skip_all = true;
            skipped.push(path.to_string_lossy().to_string());
            false
        }
        Some(OverwriteChoice::Abort) => {
            // Shared by build/add/update — keep the message operation-neutral.
            //
            // Known tradeoff: `process::exit` skips RAII unwinding, so an
            // interactive abort mid-`add`/`update` can leave the temporary git
            // clone/render directory behind (the OS temp reaper eventually
            // reclaims it). `build` has no temp dir, so it is unaffected.
            // Converting abort into an unwinding error across the shared
            // Router/Install error surfaces is a tracked follow-up; the narrow,
            // interactive-only trigger does not justify that churn here.
            eprintln!("Aborting.");
            std::process::exit(1);
        }
    }
}
