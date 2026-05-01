use ratatui::style::{Color, Modifier, Style};
use tree_sitter::Parser;

pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub style: Style,
}

pub struct Highlighter {
    parser: Parser,
}

impl Highlighter {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_bash::LANGUAGE.into())
            .expect("failed to set bash language");
        Self { parser }
    }

    pub fn highlight(&mut self, text: &str, path_binaries: &[String]) -> Vec<HighlightSpan> {
        let Some(tree) = self.parser.parse(text, None) else {
            return vec![];
        };

        let mut spans = Vec::new();
        collect_spans(tree.root_node(), text, path_binaries, &mut spans);
        spans.sort_by_key(|s| s.start);
        spans
    }
}

fn collect_spans(
    node: tree_sitter::Node,
    source: &str,
    path_binaries: &[String],
    spans: &mut Vec<HighlightSpan>,
) {
    if node.kind() == "command_name" {
        let cmd_text = &source[node.start_byte()..node.end_byte()];
        let style = if path_binaries.is_empty() || path_binaries.iter().any(|b| b == cmd_text) {
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        };
        spans.push(HighlightSpan {
            start: node.start_byte(),
            end: node.end_byte(),
            style,
        });
        return;
    }

    let style = node_style(node.kind());

    if let Some(style) = style
        && (node.child_count() == 0 || is_styled_container(node.kind()))
    {
        spans.push(HighlightSpan {
            start: node.start_byte(),
            end: node.end_byte(),
            style,
        });
        if is_styled_container(node.kind()) {
            return;
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_spans(child, source, path_binaries, spans);
    }
}

fn is_styled_container(kind: &str) -> bool {
    matches!(
        kind,
        "string" | "raw_string" | "simple_expansion" | "expansion"
    )
}

fn node_style(kind: &str) -> Option<Style> {
    match kind {
        "string" | "raw_string" => Some(Style::default().fg(Color::Green)),
        "simple_expansion" | "expansion" => Some(Style::default().fg(Color::Yellow)),
        "variable_name" => Some(Style::default().fg(Color::Yellow)),
        "file_redirect" | "heredoc_redirect" => Some(Style::default().fg(Color::Cyan)),
        "|" | "&&" | "||" => Some(Style::default().fg(Color::Cyan)),
        "comment" => Some(Style::default().fg(Color::DarkGray)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test that command name is highlighted Blue when it exists in path_binaries.
    #[test]
    fn highlight_command_name_exists() {
        let mut hl = Highlighter::new();
        let spans = hl.highlight("echo hello", &["echo".to_string()]);
        let cmd_span = spans.iter().find(|s| s.start == 0 && s.end == 4);
        assert!(cmd_span.is_some());
        assert_eq!(cmd_span.unwrap().style.fg, Some(Color::Blue));
    }

    // Test that command name is highlighted Red when it does NOT exist in path_binaries.
    #[test]
    fn highlight_command_name_not_exists() {
        let mut hl = Highlighter::new();
        let spans = hl.highlight("nonexistent hello", &["ls".to_string(), "echo".to_string()]);
        let cmd_span = spans.iter().find(|s| s.start == 0);
        assert!(cmd_span.is_some());
        assert_eq!(cmd_span.unwrap().style.fg, Some(Color::Red));
    }

    // Test that empty path_binaries treats all commands as valid (Blue).
    #[test]
    fn highlight_command_name_empty_path() {
        let mut hl = Highlighter::new();
        let spans = hl.highlight("anything hello", &[]);
        let cmd_span = spans.iter().find(|s| s.start == 0);
        assert!(cmd_span.is_some());
        assert_eq!(cmd_span.unwrap().style.fg, Some(Color::Blue));
    }

    // Test mixed: valid and invalid commands in pipeline.
    // "ls | nonexistent foo" → "ls" Blue, "nonexistent" Red
    #[test]
    fn highlight_mixed_pipeline() {
        let mut hl = Highlighter::new();
        let text = "ls | nonexistent foo";
        let spans = hl.highlight(text, &["ls".to_string()]);
        let ls_span = spans.iter().find(|s| &text[s.start..s.end] == "ls");
        let bad_span = spans
            .iter()
            .find(|s| &text[s.start..s.end] == "nonexistent");
        assert_eq!(ls_span.unwrap().style.fg, Some(Color::Blue));
        assert_eq!(bad_span.unwrap().style.fg, Some(Color::Red));
    }

    // Test that pipe operator is highlighted.
    #[test]
    fn highlight_pipe() {
        let mut hl = Highlighter::new();
        let spans = hl.highlight("ls | grep foo", &[]);
        let pipe_span = spans
            .iter()
            .find(|s| s.style.fg == Some(Color::Cyan) && &"ls | grep foo"[s.start..s.end] == "|");
        assert!(pipe_span.is_some());
    }

    // Test that string is highlighted.
    #[test]
    fn highlight_string() {
        let mut hl = Highlighter::new();
        let spans = hl.highlight("echo 'hello world'", &[]);
        let str_span = spans.iter().find(|s| s.style.fg == Some(Color::Green));
        assert!(str_span.is_some());
        let s = str_span.unwrap();
        assert_eq!(&"echo 'hello world'"[s.start..s.end], "'hello world'");
    }

    // Test that variable expansion is highlighted.
    #[test]
    fn highlight_variable() {
        let mut hl = Highlighter::new();
        let spans = hl.highlight("echo $HOME", &[]);
        let var_span = spans.iter().find(|s| s.style.fg == Some(Color::Yellow));
        assert!(var_span.is_some());
    }

    // Test that empty string returns no spans.
    #[test]
    fn highlight_empty() {
        let mut hl = Highlighter::new();
        let spans = hl.highlight("", &[]);
        assert!(spans.is_empty());
    }

    // Test highlighting with multiple commands.
    #[test]
    fn highlight_multiple_commands() {
        let mut hl = Highlighter::new();
        let text = "ls -la | grep foo";
        let spans = hl.highlight(text, &[]);
        let blue_spans: Vec<_> = spans
            .iter()
            .filter(|s| s.style.fg == Some(Color::Blue))
            .collect();
        let names: Vec<&str> = blue_spans.iter().map(|s| &text[s.start..s.end]).collect();
        assert!(names.contains(&"ls"));
        assert!(names.contains(&"grep"));
    }
}
