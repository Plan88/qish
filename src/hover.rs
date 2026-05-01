use std::path::Path;
use std::process::Command;

use crate::completion::CompletionContext;

pub struct HoverState {
    pub title: String,
    pub lines: Vec<String>,
}

/// Returns the (start, end) byte offsets of the word under the cursor.
/// A word is delimited by spaces.
pub fn word_under_cursor(text: &str, cursor: usize) -> (usize, usize) {
    let bytes = text.as_bytes();
    let len = bytes.len();

    // If cursor is at end or on a space, scan backward to find a word
    let pos = if cursor >= len || bytes[cursor] == b' ' {
        cursor.saturating_sub(1)
    } else {
        cursor
    };

    if len == 0 || bytes.get(pos).copied() == Some(b' ') {
        return (cursor, cursor);
    }

    let mut start = pos;
    while start > 0 && bytes[start - 1] != b' ' {
        start -= 1;
    }

    let mut end = pos;
    while end < len && bytes[end] != b' ' {
        end += 1;
    }

    (start, end)
}

/// Gets hover information for the given word based on context.
pub fn hover_info(word: &str, context: CompletionContext) -> Option<HoverState> {
    if word.is_empty() {
        return None;
    }

    match context {
        CompletionContext::Command => hover_command(word),
        CompletionContext::Argument => hover_path(word),
    }
}

fn hover_command(cmd: &str) -> Option<HoverState> {
    // Try `man -f <cmd>` (whatis)
    if let Ok(output) = Command::new("man").args(["-f", cmd]).output()
        && output.status.success()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<String> = text.lines().take(10).map(|l| l.to_string()).collect();
        if !lines.is_empty() {
            return Some(HoverState {
                title: cmd.to_string(),
                lines,
            });
        }
    }

    // Fallback: try `<cmd> --help`
    if let Ok(output) = Command::new(cmd).arg("--help").output() {
        let text = if output.status.success() {
            String::from_utf8_lossy(&output.stdout).into_owned()
        } else {
            String::from_utf8_lossy(&output.stderr).into_owned()
        };
        let lines: Vec<String> = text.lines().take(10).map(|l| l.to_string()).collect();
        if !lines.is_empty() {
            return Some(HoverState {
                title: format!("{cmd} --help"),
                lines,
            });
        }
    }

    None
}

fn hover_path(word: &str) -> Option<HoverState> {
    let expanded = shellexpand::tilde(word);
    let path = Path::new(expanded.as_ref());

    if path.is_dir() {
        hover_directory(path, word)
    } else if path.is_file() {
        hover_file(path, word)
    } else {
        None
    }
}

fn hover_directory(path: &Path, display_name: &str) -> Option<HoverState> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return None;
    };

    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if e.path().is_dir() {
                format!("{name}/")
            } else {
                name
            }
        })
        .collect();
    names.sort();

    let total = names.len();
    let mut lines: Vec<String> = names.into_iter().take(20).collect();
    if total > 20 {
        lines.push(format!("... and {} more", total - 20));
    }

    if lines.is_empty() {
        return None;
    }

    Some(HoverState {
        title: display_name.to_string(),
        lines,
    })
}

fn hover_file(path: &Path, display_name: &str) -> Option<HoverState> {
    let Ok(content) = std::fs::read(path) else {
        return None;
    };

    // Check if file is binary (contains null bytes in first 512 bytes)
    let check_len = content.len().min(512);
    if content[..check_len].contains(&0) {
        return Some(HoverState {
            title: display_name.to_string(),
            lines: vec!["(binary file)".to_string()],
        });
    }

    let text = String::from_utf8_lossy(&content);
    let lines: Vec<String> = text.lines().take(10).map(|l| l.to_string()).collect();

    if lines.is_empty() {
        return None;
    }

    Some(HoverState {
        title: display_name.to_string(),
        lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test word_under_cursor with cursor in the middle of a word.
    // "echo hello" cursor=6 → (5, 10) = "hello"
    #[test]
    fn word_at_middle() {
        let (start, end) = word_under_cursor("echo hello", 6);
        assert_eq!(&"echo hello"[start..end], "hello");
    }

    // Test word_under_cursor at the beginning of a word.
    // "echo hello" cursor=0 → (0, 4) = "echo"
    #[test]
    fn word_at_start() {
        let (start, end) = word_under_cursor("echo hello", 0);
        assert_eq!(&"echo hello"[start..end], "echo");
    }

    // Test word_under_cursor at end of text.
    // "echo hello" cursor=10 → (5, 10) = "hello"
    #[test]
    fn word_at_end() {
        let (start, end) = word_under_cursor("echo hello", 10);
        assert_eq!(&"echo hello"[start..end], "hello");
    }

    // Test word_under_cursor on empty text.
    #[test]
    fn word_empty_text() {
        let (start, end) = word_under_cursor("", 0);
        assert_eq!(start, 0);
        assert_eq!(end, 0);
    }

    // Test hover_info for a directory.
    #[test]
    fn hover_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("foo.txt"), "").unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();

        let result = hover_info(dir.path().to_str().unwrap(), CompletionContext::Argument);
        assert!(result.is_some());
        let state = result.unwrap();
        assert!(state.lines.iter().any(|l| l == "foo.txt"));
        assert!(state.lines.iter().any(|l| l == "subdir/"));
    }

    // Test hover_info for a file.
    #[test]
    fn hover_file() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
        writeln!(f, "line 1").unwrap();
        writeln!(f, "line 2").unwrap();

        let result = hover_info(f.path().to_str().unwrap(), CompletionContext::Argument);
        assert!(result.is_some());
        let state = result.unwrap();
        assert_eq!(state.lines[0], "line 1");
        assert_eq!(state.lines[1], "line 2");
    }

    // Test hover_info for a known command.
    #[test]
    fn hover_command_ls() {
        let result = hover_info("ls", CompletionContext::Command);
        assert!(result.is_some());
        assert!(!result.unwrap().lines.is_empty());
    }

    // Test hover_info for a nonexistent command.
    #[test]
    fn hover_nonexistent_command() {
        let result = hover_info("qish_nonexistent_cmd_12345", CompletionContext::Command);
        assert!(result.is_none());
    }

    // Test hover_info for a nonexistent path.
    #[test]
    fn hover_nonexistent_path() {
        let result = hover_info("/nonexistent/path/12345", CompletionContext::Argument);
        assert!(result.is_none());
    }
}
