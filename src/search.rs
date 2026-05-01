use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo};

pub struct FuzzySearch {
    nucleo: Nucleo<String>,
    last_pattern: String,
}

impl FuzzySearch {
    pub fn new(items: Vec<String>) -> Self {
        let nucleo = Nucleo::new(Config::DEFAULT, std::sync::Arc::new(|| {}), None, 1);

        let injector = nucleo.injector();
        for item in items {
            let _ = injector.push(item, |s, cols| {
                cols[0] = s.as_str().into();
            });
        }

        Self {
            nucleo,
            last_pattern: String::new(),
        }
    }

    pub fn query(&mut self, pattern: &str) -> Vec<String> {
        let append = pattern.starts_with(self.last_pattern.as_str());
        self.nucleo.pattern.reparse(
            0,
            pattern,
            CaseMatching::Smart,
            Normalization::Smart,
            append,
        );
        self.last_pattern = pattern.to_string();

        self.nucleo.tick(10);

        let snapshot = self.nucleo.snapshot();
        snapshot
            .matched_items(..)
            .map(|item| item.data.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test that an empty query returns all items.
    #[test]
    fn empty_query_returns_all() {
        let mut search = FuzzySearch::new(vec![
            "ls -la".to_string(),
            "git status".to_string(),
            "cargo test".to_string(),
        ]);
        let results = search.query("");
        assert_eq!(results.len(), 3);
    }

    // Test that a query filters results.
    // Given: items ["ls -la", "git status", "cargo test"]
    // When: query "git"
    // Then: "git status" is in results
    #[test]
    fn query_filters_results() {
        let mut search = FuzzySearch::new(vec![
            "ls -la".to_string(),
            "git status".to_string(),
            "cargo test".to_string(),
        ]);
        let results = search.query("git");
        assert!(results.iter().any(|r| r == "git status"));
    }

    // Test that fuzzy matching works.
    // Given: items ["cargo test", "cat file"]
    // When: query "ct"
    // Then: both match (c...t)
    #[test]
    fn fuzzy_matching() {
        let mut search = FuzzySearch::new(vec![
            "cargo test".to_string(),
            "cat file".to_string(),
            "ls".to_string(),
        ]);
        let results = search.query("ct");
        assert!(results.iter().any(|r| r == "cargo test"));
        assert!(results.iter().any(|r| r == "cat file"));
        assert!(!results.iter().any(|r| r == "ls"));
    }

    // Test that no matches returns empty.
    #[test]
    fn no_matches() {
        let mut search = FuzzySearch::new(vec!["ls".to_string(), "cd".to_string()]);
        let results = search.query("zzzzz");
        assert!(results.is_empty());
    }
}
