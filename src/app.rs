use std::path::PathBuf;

use crossterm::cursor::SetCursorStyle;
use crossterm::event::KeyCode;
use crossterm::execute;
use ratatui::DefaultTerminal;

use crate::completion::{self, CompletionContext, CompletionState};
use crate::config::{self, ConfigKeyMap};
use crate::editor::keymap::{DefaultKeyMap, KeyMap};
use crate::editor::mode::Mode;
use crate::editor::{EditorResult, EditorState};
use crate::error::AppError;
use crate::event::{AppEvent, EventReader};
use crate::highlight::Highlighter;
use crate::history;
use crate::hover;
use crate::hover::HoverState;
use crate::preview;
use crate::search::FuzzySearch;
use crate::ui::EditorWidget;

pub struct SearchState {
    pub query: String,
    pub results: Vec<String>,
    pub selected: usize,
    search: FuzzySearch,
}

impl SearchState {
    fn new(history: Vec<String>) -> Self {
        let search = FuzzySearch::new(history.clone());
        Self {
            query: String::new(),
            results: history,
            selected: 0,
            search,
        }
    }

    fn update_query(&mut self, query: String) {
        self.query = query;
        self.results = self.search.query(&self.query);
        self.selected = 0;
    }

    fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected = (self.selected + 1).min(self.results.len() - 1);
        }
    }

    fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn confirm(&self) -> Option<String> {
        self.results.get(self.selected).cloned()
    }
}

pub struct App {
    editor: EditorState,
    keymap: Box<dyn KeyMap>,
    pub search: Option<SearchState>,
    pub completion: Option<CompletionState>,
    pub hover: Option<HoverState>,
    pub config_warnings: Vec<String>,
    pub help_lines: Vec<String>,
    path_cache: Option<Vec<String>>,
    highlighter: Highlighter,
}

impl App {
    pub fn new() -> Self {
        let config_result = config::load_keymap_config();
        let has_config = config_result.config.is_some();
        let keymap: Box<dyn KeyMap> = if let Some(ref cfg) = config_result.config {
            Box::new(ConfigKeyMap::from_config(cfg))
        } else {
            Box::new(DefaultKeyMap)
        };
        let help_lines = if !has_config {
            vec![
                "Arrows: move  Tab: complete  Esc: accept  Ctrl-C: abort".to_string(),
                "Ctrl-Z/Y: undo/redo  Ctrl-R: search  Ctrl-K: hover info".to_string(),
                "Ctrl-W: del word  Ctrl-U: del to start  Ctrl-V: paste".to_string(),
                "Config: ~/.config/qish/keymap.toml  Help: qish --help".to_string(),
            ]
        } else {
            vec![]
        };
        let history = history::load_history(&Self::history_path()).unwrap_or_default();
        let mut editor = EditorState::new();
        editor.mode = Mode::Search;
        Self {
            editor,
            keymap,
            search: Some(SearchState::new(history)),
            completion: None,
            hover: None,
            config_warnings: config_result.warnings,
            help_lines,
            path_cache: Some(completion::scan_path_binaries()),
            highlighter: Highlighter::new(),
        }
    }

    fn history_path() -> PathBuf {
        dirs::home_dir().unwrap_or_default().join(".zsh_history")
    }

    fn enter_search(&mut self) {
        let history = history::load_history(&Self::history_path()).unwrap_or_default();
        self.search = Some(SearchState::new(history));
        self.editor.mode = Mode::Search;
    }

    fn exit_search(&mut self, mode: Mode) {
        self.search = None;
        self.editor.mode = mode;
    }

    fn exit_visual_if_active(&mut self) {
        if matches!(self.editor.mode, Mode::Visual) {
            self.editor.buffer.collapse_selection();
            self.editor.mode = Mode::Normal;
        }
    }

    fn path_binaries(&mut self) -> &[String] {
        if self.path_cache.is_none() {
            self.path_cache = Some(completion::scan_path_binaries());
        }
        self.path_cache.as_deref().unwrap()
    }

    fn start_completion(&mut self) {
        let text = &self.editor.buffer.text;
        let cursor = self.editor.buffer.cursor();
        let context = completion::detect_context(text, cursor);
        let (prefix_start, prefix) = completion::extract_prefix(text, cursor);

        let items = match context {
            CompletionContext::Command => {
                let binaries = self.path_binaries().to_vec();
                completion::filter_completions(&binaries, &prefix)
            }
            CompletionContext::Argument => completion::complete_paths(&prefix),
        };

        if items.is_empty() {
            return;
        }

        self.completion = Some(CompletionState {
            prefix,
            prefix_start,
            items,
            selected: 0,
        });
    }

    fn confirm_completion(&mut self) {
        if let Some(ref comp) = self.completion
            && let Some(selected) = comp.confirm()
        {
            let replacement = selected.to_string();
            let start = comp.prefix_start;
            let cursor = self.editor.buffer.cursor();
            self.editor
                .buffer
                .text
                .replace_range(start..cursor, &replacement);
            self.editor.buffer.set_cursor(start + replacement.len());
        }
        self.completion = None;
    }

    fn update_completion(&mut self) {
        let text = &self.editor.buffer.text;
        let cursor = self.editor.buffer.cursor();
        let context = completion::detect_context(text, cursor);
        let (prefix_start, prefix) = completion::extract_prefix(text, cursor);

        let items = match context {
            CompletionContext::Command => {
                let binaries = self.path_binaries().to_vec();
                completion::filter_completions(&binaries, &prefix)
            }
            CompletionContext::Argument => completion::complete_paths(&prefix),
        };

        if items.is_empty() {
            self.completion = None;
        } else if let Some(ref mut comp) = self.completion {
            comp.prefix = prefix;
            comp.prefix_start = prefix_start;
            comp.items = items;
            comp.selected = 0;
        }
    }

    fn handle_key_with_completion(&mut self, key: crossterm::event::KeyEvent) -> bool {
        let Some(ref mut comp) = self.completion else {
            return false;
        };

        match key.code {
            KeyCode::Tab | KeyCode::Down => {
                comp.select_next();
                true
            }
            KeyCode::Up => {
                comp.select_prev();
                true
            }
            KeyCode::BackTab => {
                comp.select_prev();
                true
            }
            KeyCode::Enter => {
                self.confirm_completion();
                true
            }
            KeyCode::Esc => {
                self.completion = None;
                true
            }
            KeyCode::Char(_) | KeyCode::Backspace => {
                // Let the normal keymap handle it, then update completion
                false
            }
            _ => {
                self.completion = None;
                false
            }
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<Option<String>, AppError> {
        let events = EventReader::new();

        loop {
            let path_bins = self.path_binaries().to_vec();
            let highlight_spans = self
                .highlighter
                .highlight(&self.editor.buffer.text, &path_bins);
            let preview_text = preview::expand_preview(&self.editor.buffer.text);

            terminal.draw(|frame| {
                let widget = EditorWidget {
                    state: &self.editor,
                    search: self.search.as_ref(),
                    completion: self.completion.as_ref(),
                    highlights: &highlight_spans,
                    preview: preview_text.as_deref(),
                    hover: self.hover.as_ref(),
                    warnings: &self.config_warnings,
                    help: &self.help_lines,
                };
                let area = frame.area();
                let (cursor_col, cursor_row) = widget.cursor_position(area);
                frame.render_widget(widget, area);
                frame.set_cursor_position((cursor_col, cursor_row));
            })?;

            let cursor_style = match self.editor.mode {
                Mode::Normal | Mode::Visual => SetCursorStyle::SteadyBlock,
                Mode::Insert | Mode::Search => SetCursorStyle::SteadyBar,
            };
            execute!(std::io::stdout(), cursor_style)?;

            let AppEvent::Key(key) = events.next()?;

            // If hover popup is open, dismiss on any key
            if self.hover.is_some() {
                self.hover = None;
                continue;
            }

            // If completion popup is open, intercept keys first
            if self.completion.is_some() && self.handle_key_with_completion(key) {
                continue;
            }

            if let Some(action) = self.keymap.resolve(&self.editor.mode, key) {
                match self.editor.apply(action) {
                    EditorResult::Continue => {
                        if matches!(self.editor.mode, Mode::Search) && self.search.is_none() {
                            self.enter_search();
                        }
                        // Auto-complete in insert mode: update or start completion
                        if matches!(self.editor.mode, Mode::Insert) {
                            if self.editor.buffer.text.is_empty() {
                                self.completion = None;
                            } else if self.completion.is_some() {
                                self.update_completion();
                            } else {
                                self.start_completion();
                            }
                        }
                    }
                    EditorResult::CompleteStart => {
                        self.start_completion();
                    }
                    EditorResult::HoverInfo => {
                        let text = &self.editor.buffer.text;
                        let cursor = self.editor.buffer.cursor();
                        let (start, end) = hover::word_under_cursor(text, cursor);
                        let word = &text[start..end];
                        let context = completion::detect_context(text, cursor);
                        self.hover = hover::hover_info(word, context);
                    }
                    EditorResult::ClipboardPaste => {
                        if let Some(text) = read_clipboard() {
                            self.editor.buffer.paste(&text);
                        }
                    }
                    EditorResult::ClipboardCopy => {
                        if let Some(text) = self.editor.buffer.copy_selection() {
                            copy_to_clipboard(&text);
                            self.exit_visual_if_active();
                        } else {
                            return Ok(None);
                        }
                    }
                    EditorResult::Yank => {
                        self.editor.buffer.yank();
                        copy_to_clipboard(&self.editor.buffer.register);
                        self.exit_visual_if_active();
                    }
                    EditorResult::YankDelete => {
                        self.editor.buffer.yank_delete();
                        copy_to_clipboard(&self.editor.buffer.register);
                        self.exit_visual_if_active();
                    }
                    EditorResult::PasteRegister => {
                        // Sync register from clipboard before pasting
                        if let Some(text) = read_clipboard() {
                            self.editor.buffer.register = text;
                        }
                        self.editor.buffer.paste_register();
                    }
                    EditorResult::Accept(cmd) => return Ok(Some(cmd)),
                    EditorResult::Abort => return Ok(None),
                    EditorResult::SearchInput(ch) => {
                        if let Some(ref mut s) = self.search {
                            let mut q = s.query.clone();
                            q.push(ch);
                            s.update_query(q);
                        }
                    }
                    EditorResult::SearchDeleteBack => {
                        if let Some(ref mut s) = self.search {
                            let mut q = s.query.clone();
                            q.pop();
                            s.update_query(q);
                        }
                    }
                    EditorResult::SearchSelectNext => {
                        if let Some(ref mut s) = self.search {
                            s.select_next();
                        }
                    }
                    EditorResult::SearchSelectPrev => {
                        if let Some(ref mut s) = self.search {
                            s.select_prev();
                        }
                    }
                    EditorResult::SearchConfirm => {
                        if let Some(ref s) = self.search
                            && let Some(cmd) = s.confirm()
                        {
                            self.editor.buffer.text = cmd;
                            self.editor.buffer.move_line_end();
                        }
                        self.exit_search(Mode::Normal);
                    }
                    EditorResult::SearchCancel => {
                        self.exit_search(Mode::Normal);
                    }
                }
            }
        }
    }
}

fn copy_to_clipboard(text: &str) {
    let _ = std::process::Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()
        });
}

fn read_clipboard() -> Option<String> {
    std::process::Command::new("pbpaste")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
}
