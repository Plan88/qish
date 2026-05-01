use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::mode::Mode;

#[derive(Clone)]
pub enum Action {
    InsertChar(char),
    DeleteBack,
    DeleteForward,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveWordForward,
    MoveWordBack,
    MoveLineStart,
    MoveLineEnd,
    InsertAtCursor,
    InsertAfterCursor,
    InsertAtLineStart,
    InsertAtLineEnd,
    EnterMode(Mode),
    InsertNewline,
    Accept,
    Abort,
    // Search actions
    SearchInput(char),
    SearchDeleteBack,
    SearchSelectNext,
    SearchSelectPrev,
    SearchConfirm,
    SearchCancel,
    // Completion
    CompleteStart,
    // Hover info
    HoverInfo,
    // Undo/Redo
    Undo,
    Redo,
    // Editing commands
    DeleteWordBack,
    DeleteToLineStart,
    ClearBuffer,
    ClipboardPaste,
    ClipboardCopy,
    // Yank/Paste
    Yank,
    YankDelete,
    PasteRegister,
    SelectLine,
    // Visual mode extend selection (anchor stays, head moves)
    ExtendLeft,
    ExtendRight,
    ExtendUp,
    ExtendDown,
    ExtendWordForward,
    ExtendWordBack,
    ExtendLineStart,
    ExtendLineEnd,
    // Selection-aware actions
    SelectLeft,
    SelectRight,
    SelectWordForward,
    SelectWordBack,
    DeleteSelection,
    ChangeSelection,
    CollapseSelection,
}

pub trait KeyMap {
    fn resolve(&self, mode: &Mode, key: KeyEvent) -> Option<Action>;
}

/// Default keymap: no modal editing, works like a simple text editor.
/// All editing happens in Insert mode. Ctrl-shortcuts provide advanced features.
pub struct DefaultKeyMap;

impl KeyMap for DefaultKeyMap {
    fn resolve(&self, mode: &Mode, key: KeyEvent) -> Option<Action> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return resolve_ctrl(key.code);
        }

        match mode {
            Mode::Insert => match key.code {
                KeyCode::Tab => Some(Action::CompleteStart),
                KeyCode::Char(ch) => Some(Action::InsertChar(ch)),
                KeyCode::Backspace => Some(Action::DeleteBack),
                KeyCode::Delete => Some(Action::DeleteForward),
                KeyCode::Left => Some(Action::MoveLeft),
                KeyCode::Right => Some(Action::MoveRight),
                KeyCode::Up => Some(Action::MoveUp),
                KeyCode::Down => Some(Action::MoveDown),
                KeyCode::Home => Some(Action::MoveLineStart),
                KeyCode::End => Some(Action::MoveLineEnd),
                KeyCode::Enter => Some(Action::InsertNewline),
                KeyCode::Esc => Some(Action::Accept),
                _ => None,
            },
            Mode::Normal => match key.code {
                KeyCode::Enter => Some(Action::Accept),
                KeyCode::Esc => Some(Action::Accept),
                _ => None,
            },
            Mode::Visual => match key.code {
                KeyCode::Left => Some(Action::ExtendLeft),
                KeyCode::Right => Some(Action::ExtendRight),
                KeyCode::Up => Some(Action::ExtendUp),
                KeyCode::Down => Some(Action::ExtendDown),
                KeyCode::Home => Some(Action::ExtendLineStart),
                KeyCode::End => Some(Action::ExtendLineEnd),
                KeyCode::Esc => Some(Action::EnterMode(Mode::Insert)),
                _ => None,
            },
            Mode::Search => resolve_search(key),
        }
    }
}

fn resolve_ctrl(code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Char('c') => Some(Action::ClipboardCopy), // App handles: copy if selection, else abort
        KeyCode::Char('v') => Some(Action::ClipboardPaste),
        KeyCode::Char('z') => Some(Action::Undo),
        KeyCode::Char('y') => Some(Action::Redo),
        KeyCode::Char('w') => Some(Action::DeleteWordBack),
        KeyCode::Char('u') => Some(Action::DeleteToLineStart),
        KeyCode::Char('l') => Some(Action::ClearBuffer),
        KeyCode::Char('r') => Some(Action::EnterMode(Mode::Search)),
        KeyCode::Char('k') => Some(Action::HoverInfo),
        KeyCode::Char('a') => Some(Action::MoveLineStart),
        KeyCode::Char('e') => Some(Action::MoveLineEnd),
        _ => None,
    }
}

pub fn resolve_search(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char(ch) => Some(Action::SearchInput(ch)),
        KeyCode::Backspace => Some(Action::SearchDeleteBack),
        KeyCode::Up => Some(Action::SearchSelectPrev),
        KeyCode::Down => Some(Action::SearchSelectNext),
        KeyCode::Enter => Some(Action::SearchConfirm),
        KeyCode::Esc => Some(Action::SearchCancel),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventKind;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_with(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    fn resolve(mode: &Mode, key_event: KeyEvent) -> Option<Action> {
        DefaultKeyMap.resolve(mode, key_event)
    }

    // --- Insert mode (default) ---

    // Test that typing a character inserts it.
    #[test]
    fn insert_char_input() {
        let action = resolve(&Mode::Insert, key(KeyCode::Char('a')));
        assert!(matches!(action, Some(Action::InsertChar('a'))));
    }

    // Test that Backspace deletes back.
    #[test]
    fn insert_backspace() {
        let action = resolve(&Mode::Insert, key(KeyCode::Backspace));
        assert!(matches!(action, Some(Action::DeleteBack)));
    }

    // Test that Delete deletes forward.
    #[test]
    fn insert_delete() {
        let action = resolve(&Mode::Insert, key(KeyCode::Delete));
        assert!(matches!(action, Some(Action::DeleteForward)));
    }

    // Test arrow key movement.
    #[test]
    fn insert_arrow_keys() {
        assert!(matches!(
            resolve(&Mode::Insert, key(KeyCode::Left)),
            Some(Action::MoveLeft)
        ));
        assert!(matches!(
            resolve(&Mode::Insert, key(KeyCode::Right)),
            Some(Action::MoveRight)
        ));
        assert!(matches!(
            resolve(&Mode::Insert, key(KeyCode::Up)),
            Some(Action::MoveUp)
        ));
        assert!(matches!(
            resolve(&Mode::Insert, key(KeyCode::Down)),
            Some(Action::MoveDown)
        ));
    }

    // Test Home/End for line start/end.
    #[test]
    fn insert_home_end() {
        assert!(matches!(
            resolve(&Mode::Insert, key(KeyCode::Home)),
            Some(Action::MoveLineStart)
        ));
        assert!(matches!(
            resolve(&Mode::Insert, key(KeyCode::End)),
            Some(Action::MoveLineEnd)
        ));
    }

    // Test that Enter inserts newline.
    #[test]
    fn insert_enter_newline() {
        let action = resolve(&Mode::Insert, key(KeyCode::Enter));
        assert!(matches!(action, Some(Action::InsertNewline)));
    }

    // Test that Esc accepts.
    #[test]
    fn insert_esc_accepts() {
        let action = resolve(&Mode::Insert, key(KeyCode::Esc));
        assert!(matches!(action, Some(Action::Accept)));
    }

    // Test that Tab starts completion.
    #[test]
    fn insert_tab_completes() {
        let action = resolve(&Mode::Insert, key(KeyCode::Tab));
        assert!(matches!(action, Some(Action::CompleteStart)));
    }

    // --- Ctrl shortcuts ---

    // Test Ctrl-C copies (or aborts if no selection, handled in App).
    #[test]
    fn ctrl_c_clipboard_copy() {
        let action = resolve(
            &Mode::Insert,
            key_with(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        assert!(matches!(action, Some(Action::ClipboardCopy)));
    }

    // Test Ctrl-V pastes.
    #[test]
    fn ctrl_v_clipboard_paste() {
        let action = resolve(
            &Mode::Insert,
            key_with(KeyCode::Char('v'), KeyModifiers::CONTROL),
        );
        assert!(matches!(action, Some(Action::ClipboardPaste)));
    }

    // Test Ctrl-W deletes word back.
    #[test]
    fn ctrl_w_delete_word() {
        let action = resolve(
            &Mode::Insert,
            key_with(KeyCode::Char('w'), KeyModifiers::CONTROL),
        );
        assert!(matches!(action, Some(Action::DeleteWordBack)));
    }

    // Test Ctrl-U deletes to line start.
    #[test]
    fn ctrl_u_delete_to_start() {
        let action = resolve(
            &Mode::Insert,
            key_with(KeyCode::Char('u'), KeyModifiers::CONTROL),
        );
        assert!(matches!(action, Some(Action::DeleteToLineStart)));
    }

    // Test Ctrl-L clears buffer.
    #[test]
    fn ctrl_l_clear() {
        let action = resolve(
            &Mode::Insert,
            key_with(KeyCode::Char('l'), KeyModifiers::CONTROL),
        );
        assert!(matches!(action, Some(Action::ClearBuffer)));
    }

    // Test Ctrl-Z undoes.
    #[test]
    fn ctrl_z_undoes() {
        let action = resolve(
            &Mode::Insert,
            key_with(KeyCode::Char('z'), KeyModifiers::CONTROL),
        );
        assert!(matches!(action, Some(Action::Undo)));
    }

    // Test Ctrl-Y redoes.
    #[test]
    fn ctrl_y_redoes() {
        let action = resolve(
            &Mode::Insert,
            key_with(KeyCode::Char('y'), KeyModifiers::CONTROL),
        );
        assert!(matches!(action, Some(Action::Redo)));
    }

    // Test Ctrl-R opens search.
    #[test]
    fn ctrl_r_searches() {
        let action = resolve(
            &Mode::Insert,
            key_with(KeyCode::Char('r'), KeyModifiers::CONTROL),
        );
        assert!(matches!(action, Some(Action::EnterMode(Mode::Search))));
    }

    // Test Ctrl-K shows hover info.
    #[test]
    fn ctrl_k_hover() {
        let action = resolve(
            &Mode::Insert,
            key_with(KeyCode::Char('k'), KeyModifiers::CONTROL),
        );
        assert!(matches!(action, Some(Action::HoverInfo)));
    }
}
