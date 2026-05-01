pub mod buffer;
pub mod keymap;
pub mod mode;

use buffer::Buffer;
use keymap::Action;
use mode::Mode;

pub enum EditorResult {
    Continue,
    Accept(String),
    Abort,
    SearchInput(char),
    SearchDeleteBack,
    SearchSelectNext,
    SearchSelectPrev,
    SearchConfirm,
    SearchCancel,
    CompleteStart,
    HoverInfo,
    ClipboardPaste,
    ClipboardCopy,
    Yank,
    YankDelete,
    PasteRegister,
}

pub struct EditorState {
    pub buffer: Buffer,
    pub mode: Mode,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(),
            mode: Mode::Insert,
        }
    }

    pub fn apply(&mut self, action: Action) -> EditorResult {
        match action {
            Action::InsertChar(ch) => self.buffer.insert_char(ch),
            Action::InsertNewline => self.buffer.insert_newline(),
            Action::DeleteBack => self.buffer.delete_back(),
            Action::DeleteForward => self.buffer.delete_forward(),
            Action::MoveLeft => self.buffer.move_left(),
            Action::MoveRight => self.buffer.move_right(),
            Action::MoveUp => self.buffer.move_up(),
            Action::MoveDown => self.buffer.move_down(),
            Action::MoveWordForward => self.buffer.move_word_forward(),
            Action::MoveWordBack => self.buffer.move_word_back(),
            Action::MoveLineStart => self.buffer.move_line_start(),
            Action::MoveLineEnd => self.buffer.move_line_end(),
            Action::InsertAtCursor => {
                let (start, _) = self.buffer.selection_range();
                self.buffer.set_cursor(start);
                self.mode = Mode::Insert;
            }
            Action::InsertAfterCursor => {
                let (_, end) = self.buffer.selection_range();
                self.buffer.set_cursor(end);
                self.mode = Mode::Insert;
            }
            Action::InsertAtLineStart => {
                self.buffer.collapse_selection();
                self.buffer.move_line_start();
                self.mode = Mode::Insert;
            }
            Action::InsertAtLineEnd => {
                self.buffer.collapse_selection();
                self.buffer.move_line_end();
                self.mode = Mode::Insert;
            }
            Action::EnterMode(m) => {
                self.mode = m;
            }
            Action::SelectLeft => self.buffer.select_left(),
            Action::SelectRight => self.buffer.select_right(),
            Action::SelectWordForward => self.buffer.select_word_forward(),
            Action::SelectWordBack => self.buffer.select_word_back(),
            Action::DeleteSelection => {
                self.buffer.delete_selection();
                if matches!(self.mode, Mode::Visual) {
                    self.mode = Mode::Normal;
                }
            }
            Action::ChangeSelection => {
                self.buffer.delete_selection();
                self.mode = Mode::Insert;
            }
            Action::CollapseSelection => {
                self.buffer.collapse_selection();
                if matches!(self.mode, Mode::Visual) {
                    self.mode = Mode::Normal;
                }
            }
            Action::Undo => self.buffer.undo(),
            Action::Redo => self.buffer.redo(),
            Action::DeleteWordBack => self.buffer.delete_word_back(),
            Action::DeleteToLineStart => self.buffer.delete_to_line_start(),
            Action::ClearBuffer => self.buffer.clear(),
            Action::Yank => return EditorResult::Yank,
            Action::YankDelete => return EditorResult::YankDelete,
            Action::PasteRegister => return EditorResult::PasteRegister,
            Action::SelectLine => self.buffer.select_line(),
            Action::ExtendLeft => self.buffer.extend_left(),
            Action::ExtendRight => self.buffer.extend_right(),
            Action::ExtendUp => self.buffer.extend_up(),
            Action::ExtendDown => self.buffer.extend_down(),
            Action::ExtendWordForward => self.buffer.extend_word_forward(),
            Action::ExtendWordBack => self.buffer.extend_word_back(),
            Action::ExtendLineStart => self.buffer.extend_line_start(),
            Action::ExtendLineEnd => self.buffer.extend_line_end(),
            Action::ClipboardPaste => return EditorResult::ClipboardPaste,
            Action::ClipboardCopy => return EditorResult::ClipboardCopy,
            Action::CompleteStart => return EditorResult::CompleteStart,
            Action::HoverInfo => return EditorResult::HoverInfo,
            Action::SearchInput(ch) => return EditorResult::SearchInput(ch),
            Action::SearchDeleteBack => return EditorResult::SearchDeleteBack,
            Action::SearchSelectNext => return EditorResult::SearchSelectNext,
            Action::SearchSelectPrev => return EditorResult::SearchSelectPrev,
            Action::SearchConfirm => return EditorResult::SearchConfirm,
            Action::SearchCancel => return EditorResult::SearchCancel,
            Action::Accept => return EditorResult::Accept(self.buffer.text.clone()),
            Action::Abort => return EditorResult::Abort,
        }
        EditorResult::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test that a new editor starts in insert mode with empty buffer.
    #[test]
    fn new_editor_starts_in_insert_mode() {
        let editor = EditorState::new();
        assert!(matches!(editor.mode, Mode::Insert));
        assert!(editor.buffer.text.is_empty());
    }

    // Test typing "ls -la" and accepting.
    // Given: new editor (insert mode)
    // When: type "ls -la", press Enter
    // Then: Accept with "ls -la"
    #[test]
    fn type_and_accept() {
        let mut editor = EditorState::new();
        for ch in "ls -la".chars() {
            editor.apply(Action::InsertChar(ch));
        }
        let result = editor.apply(Action::Accept);
        assert!(matches!(result, EditorResult::Accept(s) if s == "ls -la"));
    }

    // Test switching to normal mode and moving with h/l.
    // Given: editor with "abc", cursor at 3 (insert mode)
    // When: Esc → h → h → i → insert 'X'
    // Then: text is "aXbc"
    #[test]
    fn normal_mode_movement_then_insert() {
        let mut editor = EditorState::new();
        for ch in "abc".chars() {
            editor.apply(Action::InsertChar(ch));
        }
        editor.apply(Action::EnterMode(Mode::Normal));
        editor.apply(Action::MoveLeft);
        editor.apply(Action::MoveLeft);
        editor.apply(Action::InsertAtCursor);
        editor.apply(Action::InsertChar('X'));
        assert_eq!(editor.buffer.text, "aXbc");
    }

    // Test 'A' (insert at line end).
    // Given: editor with "hello", cursor at 0 (normal mode)
    // When: A → insert '!'
    // Then: text is "hello!"
    #[test]
    fn insert_at_line_end() {
        let mut editor = EditorState::new();
        for ch in "hello".chars() {
            editor.apply(Action::InsertChar(ch));
        }
        editor.apply(Action::EnterMode(Mode::Normal));
        editor.apply(Action::MoveLineStart);
        editor.apply(Action::InsertAtLineEnd);
        editor.apply(Action::InsertChar('!'));
        assert_eq!(editor.buffer.text, "hello!");
    }

    // Test 'I' (insert at line start).
    // Given: editor with "world", cursor at end (normal mode)
    // When: I → insert 'H'
    // Then: text is "Hworld"
    #[test]
    fn insert_at_line_start() {
        let mut editor = EditorState::new();
        for ch in "world".chars() {
            editor.apply(Action::InsertChar(ch));
        }
        editor.apply(Action::EnterMode(Mode::Normal));
        editor.apply(Action::InsertAtLineStart);
        editor.apply(Action::InsertChar('H'));
        assert_eq!(editor.buffer.text, "Hworld");
    }

    // Test that Abort returns EditorResult::Abort.
    #[test]
    fn abort_returns_abort() {
        let mut editor = EditorState::new();
        let result = editor.apply(Action::Abort);
        assert!(matches!(result, EditorResult::Abort));
    }

    // Test 'x' deletes character at cursor in normal mode.
    // Given: editor with "abc", normal mode, cursor at 1
    // When: x
    // Then: text is "ac"
    #[test]
    fn delete_forward_in_normal() {
        let mut editor = EditorState::new();
        for ch in "abc".chars() {
            editor.apply(Action::InsertChar(ch));
        }
        editor.apply(Action::EnterMode(Mode::Normal));
        editor.apply(Action::MoveLineStart);
        editor.apply(Action::MoveRight);
        editor.apply(Action::DeleteForward);
        assert_eq!(editor.buffer.text, "ac");
    }

    // --- Helix-style selection tests ---

    // Test Helix flow: select word then delete.
    // Given: "hello world", cursor at 0
    // When: w (select to next word) → d (delete selection)
    // Then: text is "world"
    #[test]
    fn hx_select_word_then_delete() {
        let mut editor = EditorState::new();
        for ch in "hello world".chars() {
            editor.apply(Action::InsertChar(ch));
        }
        editor.apply(Action::EnterMode(Mode::Normal));
        editor.apply(Action::CollapseSelection);
        editor.buffer.move_line_start();
        editor.apply(Action::SelectWordForward);
        editor.apply(Action::DeleteSelection);
        assert_eq!(editor.buffer.text, "world");
    }

    // Test Helix flow: select then change.
    // Given: "hello world", cursor at 0
    // When: w (select word) → c (change) → type "hi "
    // Then: text is "hi world"
    #[test]
    fn hx_select_word_then_change() {
        let mut editor = EditorState::new();
        for ch in "hello world".chars() {
            editor.apply(Action::InsertChar(ch));
        }
        editor.apply(Action::EnterMode(Mode::Normal));
        editor.apply(Action::CollapseSelection);
        editor.buffer.move_line_start();
        editor.apply(Action::SelectWordForward);
        editor.apply(Action::ChangeSelection);
        assert!(matches!(editor.mode, Mode::Insert));
        for ch in "hi ".chars() {
            editor.apply(Action::InsertChar(ch));
        }
        assert_eq!(editor.buffer.text, "hi world");
    }

    // Test Helix flow: each l creates a 1-char selection at destination, then collapse.
    // Given: "abc", cursor at 0
    // When: l → l → ;
    // Then: after first l: head=1, anchor=2 (1-char selection on 'b')
    //       after second l: head=2, anchor=3 (1-char selection on 'c')
    //       after ;: anchor=2, head=2
    #[test]
    fn hx_select_then_collapse() {
        let mut editor = EditorState::new();
        for ch in "abc".chars() {
            editor.apply(Action::InsertChar(ch));
        }
        editor.apply(Action::EnterMode(Mode::Normal));
        editor.apply(Action::CollapseSelection);
        editor.buffer.move_line_start();
        editor.apply(Action::SelectRight);
        assert_eq!(editor.buffer.selections[0].head, 1);
        assert_eq!(editor.buffer.selections[0].anchor, 2);
        editor.apply(Action::SelectRight);
        assert_eq!(editor.buffer.selections[0].head, 2);
        assert_eq!(editor.buffer.selections[0].anchor, 3);
        editor.apply(Action::CollapseSelection);
        assert_eq!(editor.buffer.selections[0].anchor, 2);
        assert_eq!(editor.buffer.selections[0].head, 2);
    }
}
