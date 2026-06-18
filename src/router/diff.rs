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
        || DiffOutput {
            header: format!("--- /dev/null\n+++ {path_display}\n"),
            hunks: format!("+{rendered}"),
            stats: DiffStats {
                additions: rendered.lines().count(),
                deletions: 0,
                is_new_file: true,
            },
        },
        |old| compute_unified_diff(old, rendered, path_display),
    )
}

fn compute_unified_diff(old: &str, new: &str, path_display: &str) -> DiffOutput {
    let diff = TextDiff::from_lines(old, new);
    let mut hunks = String::new();
    let mut additions = 0;
    let mut deletions = 0;

    for op in diff.ops() {
        for change in diff.iter_changes(op) {
            let sign = match change.tag() {
                ChangeTag::Delete => {
                    deletions += 1;
                    '-'
                }
                ChangeTag::Insert => {
                    additions += 1;
                    '+'
                }
                ChangeTag::Equal => continue,
            };
            hunks.push(sign);
            hunks.push_str(change.value());
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
        header: format!("--- a/{path_display}\n+++ b/{path_display}\n"),
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
        assert!(output.hunks.contains("+hello world\n"));
        assert!(output.stats.is_new_file);
        assert_eq!(output.stats.additions, 1);
        assert_eq!(output.stats.deletions, 0);
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
        assert!(output.hunks.contains("+new content\n"));
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
