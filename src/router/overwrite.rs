use std::io::{self, IsTerminal, Write};
use std::path::Path;

/// User response to an overwrite prompt.
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
pub(super) fn prompt_overwrite(path: &Path) -> Option<OverwriteChoice> {
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

/// Unified overwrite decision combining the force/skip-all guard and interactive prompt.
///
/// Returns `true` if the caller should write the file, `false` if it should skip.
/// Handles `skip_all` progression and abort exit internally.
pub(super) fn resolve_overwrite(
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
            eprintln!("Aborting build.");
            std::process::exit(1);
        }
    }
}
