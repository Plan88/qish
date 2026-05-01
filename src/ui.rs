use ratatui::buffer::Buffer as RatBuf;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Widget};
use unicode_width::UnicodeWidthStr;

use crate::app::SearchState;
use crate::completion::CompletionState;
use crate::editor::EditorState;
use crate::editor::mode::Mode;
use crate::highlight::HighlightSpan;
use crate::hover::HoverState;

pub struct EditorWidget<'a> {
    pub state: &'a EditorState,
    pub search: Option<&'a SearchState>,
    pub completion: Option<&'a CompletionState>,
    pub highlights: &'a [HighlightSpan],
    pub preview: Option<&'a str>,
    pub hover: Option<&'a HoverState>,
    pub warnings: &'a [String],
    pub help: &'a [String],
}

impl Widget for EditorWidget<'_> {
    fn render(self, area: Rect, buf: &mut RatBuf) {
        // Render warnings and help at the bottom of the screen
        let bottom_lines = (self.warnings.len() + self.help.len()) as u16;
        let main_area = if bottom_lines > 0 && area.height > bottom_lines {
            let mut y = area.y + area.height - bottom_lines;
            for help in self.help.iter() {
                let line = Line::from(Span::styled(
                    help.as_str(),
                    Style::default().fg(Color::DarkGray),
                ));
                line.render(Rect::new(area.x, y, area.width, 1), buf);
                y += 1;
            }
            for warning in self.warnings.iter() {
                let line = Line::from(Span::styled(
                    warning.as_str(),
                    Style::default().fg(Color::Red),
                ));
                line.render(Rect::new(area.x, y, area.width, 1), buf);
                y += 1;
            }
            Rect::new(area.x, area.y, area.width, area.height - bottom_lines)
        } else {
            area
        };

        if let Some(search) = self.search {
            self.render_search(search, main_area, buf);
        } else {
            self.render_editor(main_area, buf);
            let editor_lines = self.state.buffer.text.split('\n').count() as u16;
            let mut popup_y = main_area.y + editor_lines;
            if let Some(preview) = self.preview
                && main_area.height > editor_lines + 2
            {
                let preview_height = 3u16;
                let preview_area = Rect::new(
                    main_area.x,
                    popup_y,
                    main_area.width,
                    preview_height.min(main_area.height - editor_lines),
                );
                self.render_preview(preview, preview_area, buf);
                popup_y += preview_height;
            }
            if let Some(hover) = self.hover {
                let remaining = (main_area.y + main_area.height).saturating_sub(popup_y);
                if remaining > 2 {
                    let hover_area = Rect::new(main_area.x, popup_y, main_area.width, remaining);
                    self.render_hover(hover, hover_area, buf);
                }
            } else if let Some(comp) = self.completion {
                let remaining = (main_area.y + main_area.height).saturating_sub(popup_y);
                if remaining > 2 {
                    let comp_area = Rect::new(main_area.x, popup_y, main_area.width, remaining);
                    self.render_completion(comp, comp_area, buf);
                }
            }
        }
    }
}

impl EditorWidget<'_> {
    fn render_editor(&self, area: Rect, buf: &mut RatBuf) {
        let mode_label = match self.state.mode {
            Mode::Normal => Span::styled(
                " NORMAL ",
                Style::default().bg(Color::Blue).fg(Color::White),
            ),
            Mode::Insert => Span::styled(
                " INSERT ",
                Style::default().bg(Color::Green).fg(Color::Black),
            ),
            Mode::Visual => Span::styled(
                " VISUAL ",
                Style::default().bg(Color::Yellow).fg(Color::Black),
            ),
            Mode::Search => Span::styled(
                " SEARCH ",
                Style::default().bg(Color::Magenta).fg(Color::White),
            ),
        };

        let text = &self.state.buffer.text;
        let text_lines: Vec<&str> = text.split('\n').collect();

        for (i, text_line) in text_lines.iter().enumerate() {
            let y = area.y + i as u16;
            if y >= area.y + area.height {
                break;
            }

            if i == 0 {
                let separator = Span::raw(" ");
                let mut spans = vec![mode_label.clone(), separator];
                let line_start = 0;
                let line_end = text_line.len();
                spans.extend(self.build_spans_for_range(text, line_start, line_end));
                let line = Line::from(spans);
                line.render(Rect::new(area.x, y, area.width, 1), buf);
            } else {
                let line_start = text_lines[..i]
                    .iter()
                    .map(|l| l.len() + 1) // +1 for '\n'
                    .sum::<usize>();
                let line_end = line_start + text_line.len();
                // Pad continuation lines to align with first line's text start
                let padding = Span::raw(" ".repeat(9)); // same width as mode label + separator
                let mut spans = vec![padding];
                spans.extend(self.build_spans_for_range(text, line_start, line_end));
                let line = Line::from(spans);
                line.render(Rect::new(area.x, y, area.width, 1), buf);
            }
        }
    }

    fn build_spans_for_range<'a>(
        &self,
        full_text: &'a str,
        range_start: usize,
        range_end: usize,
    ) -> Vec<Span<'a>> {
        let text_slice = &full_text[range_start..range_end];

        if self.state.buffer.has_selection() {
            let (sel_start, sel_end) = self.state.buffer.selection_range();
            // Clip selection to this line range
            let s = sel_start.max(range_start).min(range_end);
            let e = sel_end.max(range_start).min(range_end);
            if s < e {
                let mut spans = Vec::new();
                if s > range_start {
                    spans.push(Span::raw(&full_text[range_start..s]));
                }
                spans.push(Span::styled(
                    &full_text[s..e],
                    Style::default().bg(Color::DarkGray).fg(Color::White),
                ));
                if e < range_end {
                    spans.push(Span::raw(&full_text[e..range_end]));
                }
                return spans;
            }
        }

        // Use syntax highlighting
        if self.highlights.is_empty() || text_slice.is_empty() {
            return vec![Span::raw(text_slice)];
        }

        let mut spans = Vec::new();
        let mut pos = range_start;

        for hl in self.highlights {
            if hl.end <= range_start || hl.start >= range_end {
                continue;
            }
            let hl_start = hl.start.max(range_start);
            let hl_end = hl.end.min(range_end);
            if hl_start > pos {
                spans.push(Span::raw(&full_text[pos..hl_start]));
            }
            if hl_start < hl_end {
                spans.push(Span::styled(&full_text[hl_start..hl_end], hl.style));
                pos = hl_end;
            }
        }

        if pos < range_end {
            spans.push(Span::raw(&full_text[pos..range_end]));
        }

        spans
    }

    fn render_preview(&self, preview: &str, area: Rect, buf: &mut RatBuf) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(
                " Preview ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height > 0 {
            let line = Line::from(Span::styled(preview, Style::default().fg(Color::DarkGray)));
            line.render(inner, buf);
        }
    }

    fn render_search(&self, search: &SearchState, area: Rect, buf: &mut RatBuf) {
        let prompt = Span::styled(
            " SEARCH ",
            Style::default().bg(Color::Magenta).fg(Color::White),
        );
        let separator = Span::raw(" ");
        let query = Span::raw(&search.query);
        let prompt_line = Line::from(vec![prompt, separator, query]);
        if area.height > 0 {
            prompt_line.render(Rect::new(area.x, area.y, area.width, 1), buf);
        }

        if area.height > 3 {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(
                    " History ",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ));
            let list_area = Rect::new(area.x, area.y + 1, area.width, area.height - 1);
            let inner = block.inner(list_area);
            block.render(list_area, buf);

            for (i, result) in search
                .results
                .iter()
                .take(inner.height as usize)
                .enumerate()
            {
                let y = inner.y + i as u16;
                if y >= inner.y + inner.height {
                    break;
                }
                let style = if i == search.selected {
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let prefix = if i == search.selected { "> " } else { "  " };
                let line = Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(result.as_str(), style),
                ]);
                line.render(Rect::new(inner.x, y, inner.width, 1), buf);
            }
        }
    }

    fn render_completion(&self, comp: &CompletionState, area: Rect, buf: &mut RatBuf) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(
                " Completion ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        block.render(area, buf);

        let max_items = (inner.height as usize).min(10);
        for (i, item) in comp.items.iter().take(max_items).enumerate() {
            let y = inner.y + i as u16;
            if y >= inner.y + inner.height {
                break;
            }
            let style = if i == comp.selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            let prefix = if i == comp.selected { "> " } else { "  " };
            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(item.as_str(), style),
            ]);
            line.render(Rect::new(inner.x, y, inner.width, 1), buf);
        }
    }

    fn render_hover(&self, hover: &HoverState, area: Rect, buf: &mut RatBuf) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(
                format!(" {} ", hover.title),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        block.render(area, buf);

        let max_lines = (inner.height as usize).min(12);
        for (i, content) in hover.lines.iter().take(max_lines).enumerate() {
            let y = inner.y + i as u16;
            if y >= inner.y + inner.height {
                break;
            }
            let line = Line::from(Span::styled(
                content.as_str(),
                Style::default().fg(Color::Gray),
            ));
            line.render(Rect::new(inner.x, y, inner.width, 1), buf);
        }
    }

    /// Returns (col, row) for the cursor position in the terminal.
    pub fn cursor_position(&self, area: Rect) -> (u16, u16) {
        let mode_label_width: u16 = 9; // " XXXXXX " + " "
        if self.search.is_some() {
            let query_width = self.search.map(|s| s.query.width() as u16).unwrap_or(0);
            (area.x + mode_label_width + query_width, area.y)
        } else {
            let text = &self.state.buffer.text;
            let cursor = self.state.buffer.cursor();
            let before_cursor = &text[..cursor];

            // Find which line the cursor is on and the display width within that line
            let row = before_cursor.matches('\n').count() as u16;
            let current_line = before_cursor
                .rfind('\n')
                .map(|i| &before_cursor[i + 1..])
                .unwrap_or(before_cursor);
            let col = current_line.width() as u16;

            // All lines have the same offset (mode label width) to stay aligned
            (area.x + mode_label_width + col, area.y + row)
        }
    }
}
