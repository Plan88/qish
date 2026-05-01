use std::path::Path;

use nucleo_matcher::pattern::{Atom, AtomKind, CaseMatching, Normalization};
use nucleo_matcher::{Config, Matcher, Utf32Str};

pub enum CompletionContext {
    Command,
    Argument,
}

pub struct CompletionState {
    pub prefix: String,
    pub prefix_start: usize,
    pub items: Vec<String>,
    pub selected: usize,
}

impl CompletionState {
    pub fn select_next(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1) % self.items.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + self.items.len() - 1) % self.items.len();
        }
    }

    pub fn confirm(&self) -> Option<&str> {
        self.items.get(self.selected).map(|s| s.as_str())
    }
}

/// Detects whether the cursor is at command position (first word) or argument position.
pub fn detect_context(text: &str, cursor: usize) -> CompletionContext {
    let before_cursor = &text[..cursor];
    let trimmed = before_cursor.trim_start();
    if trimmed.contains(' ') {
        CompletionContext::Argument
    } else {
        CompletionContext::Command
    }
}

/// Extracts the prefix being completed and its starting byte offset in the buffer.
pub fn extract_prefix(text: &str, cursor: usize) -> (usize, String) {
    let before_cursor = &text[..cursor];
    let start = before_cursor.rfind(' ').map(|i| i + 1).unwrap_or(0);
    (start, before_cursor[start..].to_string())
}

/// Filters candidates by fuzzy matching against the prefix.
/// Results are sorted by match score (best first).
pub fn filter_completions(candidates: &[String], prefix: &str) -> Vec<String> {
    if prefix.is_empty() {
        return candidates.to_vec();
    }

    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Atom::new(
        prefix,
        CaseMatching::Smart,
        Normalization::Smart,
        AtomKind::Fuzzy,
        false,
    );

    let mut buf = Vec::new();
    let mut scored: Vec<(_, &String)> = candidates
        .iter()
        .filter_map(|c| {
            let haystack = Utf32Str::new(c, &mut buf);
            let score = pattern.score(haystack, &mut matcher)?;
            Some((score, c))
        })
        .collect();

    scored.sort_by_key(|b| std::cmp::Reverse(b.0));
    scored.into_iter().map(|(_, s)| s.clone()).collect()
}

/// Lists files/directories in the directory part of the prefix for path completion.
pub fn complete_paths(prefix: &str) -> Vec<String> {
    let (dir, file_prefix) = if prefix.contains('/') {
        let last_slash = prefix.rfind('/').unwrap();
        let dir_part = &prefix[..=last_slash];
        let file_part = &prefix[last_slash + 1..];
        (
            shellexpand::tilde(dir_part).into_owned(),
            file_part.to_string(),
        )
    } else {
        (".".to_string(), prefix.to_string())
    };

    let dir_path = Path::new(&dir);
    let Ok(entries) = std::fs::read_dir(dir_path) else {
        return Vec::new();
    };

    let mut results = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&file_prefix) {
            let display = if prefix.contains('/') {
                let dir_prefix = &prefix[..=prefix.rfind('/').unwrap()];
                if entry.path().is_dir() {
                    format!("{dir_prefix}{name}/")
                } else {
                    format!("{dir_prefix}{name}")
                }
            } else if entry.path().is_dir() {
                format!("{name}/")
            } else {
                name
            };
            results.push(display);
        }
    }
    results.sort();
    results
}

/// Scans $PATH directories for executable binaries.
pub fn scan_path_binaries() -> Vec<String> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    let mut binaries = std::collections::HashSet::new();

    for dir in path_var.split(':') {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type()
                && (ft.is_file() || ft.is_symlink())
            {
                binaries.insert(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }

    let mut result: Vec<String> = binaries.into_iter().collect();
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test that cursor within first word is Command context.
    #[test]
    fn context_first_word() {
        assert!(matches!(
            detect_context("ls", 2),
            CompletionContext::Command
        ));
    }

    // Test that empty string is Command context.
    #[test]
    fn context_empty() {
        assert!(matches!(detect_context("", 0), CompletionContext::Command));
    }

    // Test that cursor after space is Argument context.
    #[test]
    fn context_after_space() {
        assert!(matches!(
            detect_context("ls -", 4),
            CompletionContext::Argument
        ));
    }

    // Test that cursor in second word is Argument context.
    #[test]
    fn context_second_word() {
        assert!(matches!(
            detect_context("git stat", 8),
            CompletionContext::Argument
        ));
    }

    // Test prefix extraction at start of line.
    #[test]
    fn prefix_at_start() {
        let (start, prefix) = extract_prefix("ls", 2);
        assert_eq!(start, 0);
        assert_eq!(prefix, "ls");
    }

    // Test prefix extraction in second word.
    #[test]
    fn prefix_second_word() {
        let (start, prefix) = extract_prefix("git sta", 7);
        assert_eq!(start, 4);
        assert_eq!(prefix, "sta");
    }

    // Test prefix extraction with empty prefix (cursor right after space).
    #[test]
    fn prefix_empty_after_space() {
        let (start, prefix) = extract_prefix("ls ", 3);
        assert_eq!(start, 3);
        assert_eq!(prefix, "");
    }

    // Test filter with matching prefix.
    #[test]
    fn filter_matches() {
        let candidates = vec!["cargo".to_string(), "cat".to_string(), "ls".to_string()];
        let results = filter_completions(&candidates, "ca");
        assert!(results.contains(&"cargo".to_string()));
        assert!(results.contains(&"cat".to_string()));
        assert!(!results.contains(&"ls".to_string()));
    }

    // Test filter with empty prefix returns all.
    #[test]
    fn filter_empty_prefix() {
        let candidates = vec!["a".to_string(), "b".to_string()];
        let results = filter_completions(&candidates, "");
        assert_eq!(results.len(), 2);
    }

    // Test filter with no matches.
    #[test]
    fn filter_no_matches() {
        let candidates = vec!["ls".to_string()];
        let results = filter_completions(&candidates, "zz");
        assert!(results.is_empty());
    }

    // Test that fuzzy matching works: "lippy" matches "cargo-clippy".
    #[test]
    fn filter_fuzzy() {
        let candidates = vec![
            "cargo-clippy".to_string(),
            "cargo-fmt".to_string(),
            "cargo-test".to_string(),
        ];
        let results = filter_completions(&candidates, "lippy");
        assert!(results.contains(&"cargo-clippy".to_string()));
        assert!(!results.contains(&"cargo-fmt".to_string()));
    }

    // Test path completion with a temp directory.
    #[test]
    fn complete_paths_in_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("foo.txt"), "").unwrap();
        std::fs::write(dir.path().join("bar.txt"), "").unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();

        let prefix = format!("{}/", dir.path().display());
        let results = complete_paths(&prefix);
        assert!(results.iter().any(|r| r.ends_with("foo.txt")));
        assert!(results.iter().any(|r| r.ends_with("bar.txt")));
        assert!(results.iter().any(|r| r.ends_with("subdir/")));
    }

    // Test path completion with partial filename.
    #[test]
    fn complete_paths_partial() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("foo.txt"), "").unwrap();
        std::fs::write(dir.path().join("bar.txt"), "").unwrap();

        let prefix = format!("{}/fo", dir.path().display());
        let results = complete_paths(&prefix);
        assert_eq!(results.len(), 1);
        assert!(results[0].ends_with("foo.txt"));
    }

    // Test scan_path_binaries returns non-empty (assumes $PATH has something).
    #[test]
    fn scan_path_returns_binaries() {
        let binaries = scan_path_binaries();
        assert!(!binaries.is_empty());
        assert!(binaries.iter().any(|b| b == "ls"));
    }

    // Test CompletionState navigation.
    #[test]
    fn state_navigation() {
        let mut state = CompletionState {
            prefix: String::new(),
            prefix_start: 0,
            items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            selected: 0,
        };
        state.select_next();
        assert_eq!(state.selected, 1);
        state.select_next();
        assert_eq!(state.selected, 2);
        state.select_next();
        assert_eq!(state.selected, 0); // wraps
        state.select_prev();
        assert_eq!(state.selected, 2); // wraps back
    }

    // Test CompletionState confirm returns selected item.
    #[test]
    fn state_confirm() {
        let state = CompletionState {
            prefix: String::new(),
            prefix_start: 0,
            items: vec!["cargo".to_string(), "cat".to_string()],
            selected: 1,
        };
        assert_eq!(state.confirm(), Some("cat"));
    }
}
