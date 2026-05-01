use std::path::Path;

use crate::error::AppError;

/// Reads shell history from a file and returns commands in reverse order (most recent first).
/// Supports both plain format (one command per line) and zsh extended format (`: timestamp:0;command`).
/// Duplicates are removed, keeping the most recent occurrence.
pub fn load_history(path: &Path) -> Result<Vec<String>, AppError> {
    let content = std::fs::read_to_string(path)?;
    let mut seen = std::collections::HashSet::new();
    let mut commands: Vec<String> = Vec::new();

    for line in content.lines().rev() {
        let cmd = parse_history_line(line);
        if !cmd.is_empty() && seen.insert(cmd.to_string()) {
            commands.push(cmd.to_string());
        }
    }

    Ok(commands)
}

fn parse_history_line(line: &str) -> &str {
    // zsh extended format: ": 1234567890:0;actual command"
    if let Some(rest) = line.strip_prefix(": ")
        && let Some(pos) = rest.find(';')
    {
        return rest[pos + 1..].trim();
    }
    line.trim()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Test parsing plain history format.
    // Given: a file with one command per line
    // When: load_history
    // Then: returns commands in reverse order
    #[test]
    fn load_plain_history() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "ls").unwrap();
        writeln!(f, "cd /tmp").unwrap();
        writeln!(f, "echo hello").unwrap();

        let history = load_history(f.path()).unwrap();
        assert_eq!(history, vec!["echo hello", "cd /tmp", "ls"]);
    }

    // Test parsing zsh extended history format.
    // Given: a file with ": timestamp:0;command" lines
    // When: load_history
    // Then: extracts commands, most recent first
    #[test]
    fn load_extended_history() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, ": 1700000000:0;ls -la").unwrap();
        writeln!(f, ": 1700000001:0;git status").unwrap();

        let history = load_history(f.path()).unwrap();
        assert_eq!(history, vec!["git status", "ls -la"]);
    }

    // Test that duplicates are removed, keeping most recent.
    // Given: "ls" appears twice
    // When: load_history
    // Then: "ls" appears only once (from the most recent occurrence)
    #[test]
    fn dedup_keeps_most_recent() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "ls").unwrap();
        writeln!(f, "cd /tmp").unwrap();
        writeln!(f, "ls").unwrap();

        let history = load_history(f.path()).unwrap();
        assert_eq!(history, vec!["ls", "cd /tmp"]);
    }

    // Test that empty lines are skipped.
    #[test]
    fn skip_empty_lines() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "ls").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "cd").unwrap();

        let history = load_history(f.path()).unwrap();
        assert_eq!(history, vec!["cd", "ls"]);
    }

    // Test that missing file returns an error.
    #[test]
    fn missing_file_returns_error() {
        let result = load_history(Path::new("/nonexistent/history"));
        assert!(result.is_err());
    }
}
