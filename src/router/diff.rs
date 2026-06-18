use std::fmt::Write as _;
use std::path::Path;

use similar::{ChangeTag, TextDiff};

pub struct DiffOutput {
    pub header: String,
    pub hunks: String,
    pub stats: DiffStats,
}

pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub is_new_file: bool,
}

pub fn compute_diff(existing: Option<&str>, rendered: &str, path_display: &str) -> DiffOutput {
    existing.map_or_else(
        || new_file_diff(rendered, path_display),
        |old| compute_unified_diff(old, rendered, path_display),
    )
}

fn new_file_diff(rendered: &str, path_display: &str) -> DiffOutput {
    let mut hunks = String::new();
    for line in rendered.lines() {
        let _ = writeln!(hunks, "\x1b[32m+{line}\x1b[0m");
    }
    let additions = rendered.lines().count();
    DiffOutput {
        header: format!("\x1b[1m--- /dev/null\x1b[0m\n\x1b[1m+++ {path_display}\x1b[0m\n"),
        hunks,
        stats: DiffStats {
            additions,
            deletions: 0,
            is_new_file: true,
        },
    }
}

fn compute_unified_diff(old: &str, new: &str, path_display: &str) -> DiffOutput {
    let diff = TextDiff::from_lines(old, new);
    let mut hunks = String::new();
    let mut additions = 0;
    let mut deletions = 0;

    for op in diff.ops() {
        for change in diff.iter_changes(op) {
            let (sign, color) = match change.tag() {
                ChangeTag::Delete => {
                    deletions += 1;
                    ('-', "\x1b[31m")
                }
                ChangeTag::Insert => {
                    additions += 1;
                    ('+', "\x1b[32m")
                }
                ChangeTag::Equal => continue,
            };
            let value = change.value();
            for line in value.lines() {
                let _ = writeln!(hunks, "{color}{sign}{line}\x1b[0m");
            }
        }
    }

    if hunks.is_empty() {
        return DiffOutput {
            header: String::new(),
            hunks: String::new(),
            stats: DiffStats {
                additions: 0,
                deletions: 0,
                is_new_file: false,
            },
        };
    }

    DiffOutput {
        header: format!("\x1b[1m--- a/{path_display}\x1b[0m\n\x1b[1m+++ b/{path_display}\x1b[0m\n"),
        hunks,
        stats: DiffStats {
            additions,
            deletions,
            is_new_file: false,
        },
    }
}

pub fn read_existing(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_file_shows_full_addition() {
        let output = compute_diff(None, "hello world\n", "test/SKILL.md");
        assert!(output.header.contains("/dev/null"));
        assert!(output.hunks.contains("+hello world"));
        assert!(output.stats.is_new_file);
        assert_eq!(output.stats.additions, 1);
        assert_eq!(output.stats.deletions, 0);
    }

    #[test]
    fn new_file_multi_line_prefixed_per_line() {
        let output = compute_diff(None, "line1\nline2\nline3\n", "test/SKILL.md");
        assert!(output.hunks.contains("+line1"));
        assert!(output.hunks.contains("+line2"));
        assert!(output.hunks.contains("+line3"));
        assert_eq!(output.stats.additions, 3);
    }

    #[test]
    fn unchanged_content_produces_empty_diff() {
        let output = compute_diff(Some("same\n"), "same\n", "test/SKILL.md");
        assert!(output.header.is_empty());
        assert!(output.hunks.is_empty());
        assert_eq!(output.stats.additions, 0);
        assert_eq!(output.stats.deletions, 0);
    }

    #[test]
    fn changed_content_shows_additions_and_removals() {
        let output = compute_diff(
            Some("line one\nline two\n"),
            "line one\nline three\n",
            "test/SKILL.md",
        );
        assert!(output.header.contains("a/test/SKILL.md"));
        assert!(output.hunks.contains("-line two"));
        assert!(output.hunks.contains("+line three"));
        assert_eq!(output.stats.additions, 1);
        assert_eq!(output.stats.deletions, 1);
    }

    #[test]
    fn empty_old_content_shows_all_additions() {
        let output = compute_diff(Some(""), "new content\n", "test/SKILL.md");
        assert!(output.hunks.contains("+new content"));
        assert_eq!(output.stats.additions, 1);
        assert_eq!(output.stats.deletions, 0);
    }

    #[test]
    fn read_existing_returns_none_for_missing_file() {
        assert!(read_existing(Path::new("/nonexistent/path/file.md")).is_none());
    }

    #[test]
    fn read_existing_returns_content_for_existing_file() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("diff_read");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.md"), "content").unwrap();

        let content = read_existing(&dir.join("test.md"));
        assert_eq!(content.as_deref(), Some("content"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
