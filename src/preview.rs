use std::path::Path;

/// Expands `~`, `$VAR`, and relative paths in the text.
/// Returns `Some(expanded)` if the result differs from the input, `None` otherwise.
pub fn expand_preview(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }

    let expanded: String = text
        .split(' ')
        .map(|word| {
            // First expand ~ and $VAR
            let tilde = shellexpand::tilde(word);
            let env_expanded = shellexpand::env(&tilde)
                .map(|e| e.into_owned())
                .unwrap_or_else(|_| tilde.into_owned());

            // Then resolve relative paths to absolute
            expand_relative_path(&env_expanded)
        })
        .collect::<Vec<_>>()
        .join(" ");

    if expanded == text {
        None
    } else {
        Some(expanded)
    }
}

/// If the word looks like a path (contains `/` or starts with `.`), try to resolve it.
fn expand_relative_path(word: &str) -> String {
    if !looks_like_path(word) {
        return word.to_string();
    }

    let path = Path::new(word);
    if let Ok(absolute) = std::fs::canonicalize(path) {
        let resolved = absolute.to_string_lossy().into_owned();
        // Append trailing / if the original had it or the path is a directory
        if word.ends_with('/') && !resolved.ends_with('/') {
            format!("{resolved}/")
        } else {
            resolved
        }
    } else {
        word.to_string()
    }
}

fn looks_like_path(word: &str) -> bool {
    word.starts_with("./") || word.starts_with("../") || word == "." || word == ".."
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test that ~ is expanded to home directory.
    #[test]
    fn expand_tilde() {
        let result = expand_preview("ls ~/Desktop");
        assert!(result.is_some());
        let expanded = result.unwrap();
        assert!(!expanded.contains('~'));
        assert!(expanded.contains("Desktop"));
    }

    // Test that $HOME is expanded.
    #[test]
    fn expand_env_var() {
        let result = expand_preview("echo $HOME");
        assert!(result.is_some());
        let home = std::env::var("HOME").unwrap();
        assert!(result.unwrap().contains(&home));
    }

    // Test that text without expansions returns None.
    #[test]
    fn no_expansion_needed() {
        let result = expand_preview("ls -la");
        assert!(result.is_none());
    }

    // Test that empty text returns None.
    #[test]
    fn empty_text() {
        let result = expand_preview("");
        assert!(result.is_none());
    }

    // Test that unknown variable is not expanded (returns None or original).
    #[test]
    fn unknown_var() {
        let result = expand_preview("echo $QISH_NONEXISTENT_VAR_12345");
        assert!(result.is_none());
    }

    // Test that relative path ./ is expanded to absolute.
    #[test]
    fn expand_relative_dot() {
        let result = expand_preview("ls ./");
        assert!(result.is_some());
        let expanded = result.unwrap();
        assert!(expanded.starts_with("ls /"));
        assert!(expanded.ends_with('/'));
    }

    // Test that relative path ../ is expanded to absolute.
    #[test]
    fn expand_relative_dotdot() {
        let result = expand_preview("cd ../");
        assert!(result.is_some());
        let expanded = result.unwrap();
        assert!(expanded.starts_with("cd /"));
    }

    // Test that nonexistent relative path is not expanded.
    #[test]
    fn expand_nonexistent_relative() {
        let result = expand_preview("ls ./nonexistent_dir_12345");
        assert!(result.is_none());
    }

    // Test that . alone is expanded.
    #[test]
    fn expand_dot_alone() {
        let result = expand_preview("ls .");
        assert!(result.is_some());
        let expanded = result.unwrap();
        assert!(expanded.starts_with("ls /"));
    }
}
