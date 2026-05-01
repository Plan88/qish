use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};
use serde::Deserialize;

use crate::editor::keymap::{Action, KeyMap};
use crate::editor::mode::Mode;

#[derive(Deserialize)]
pub struct KeymapConfig {
    #[serde(default)]
    pub normal: HashMap<String, String>,
    #[serde(default)]
    pub insert: HashMap<String, String>,
    #[serde(default)]
    pub visual: HashMap<String, String>,
}

pub struct ConfigLoadResult {
    pub config: Option<KeymapConfig>,
    pub warnings: Vec<String>,
}

pub fn load_keymap_config() -> ConfigLoadResult {
    let paths = [
        dirs::home_dir().map(|h| h.join(".config/qish/keymap.toml")),
        dirs::config_dir().map(|c| c.join("qish").join("keymap.toml")),
    ];

    for path in paths.into_iter().flatten() {
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let config: KeymapConfig = match toml::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                return ConfigLoadResult {
                    config: None,
                    warnings: vec![format!("Failed to parse {}: {}", path.display(), e)],
                };
            }
        };

        let warnings = validate_config(&config);
        return ConfigLoadResult {
            config: Some(config),
            warnings,
        };
    }

    ConfigLoadResult {
        config: None,
        warnings: vec![],
    }
}

fn validate_config(config: &KeymapConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    for (section, map) in [
        ("normal", &config.normal),
        ("insert", &config.insert),
        ("visual", &config.visual),
    ] {
        for (action_name, key_str) in map {
            if parse_key(key_str).is_none() {
                warnings.push(format!("[{section}] unknown key: \"{key_str}\""));
            }
            if parse_action(action_name, &Mode::Normal).is_none() {
                warnings.push(format!("[{section}] unknown action: \"{action_name}\""));
            }
        }
    }
    warnings
}

pub fn parse_key(key_str: &str) -> Option<(KeyCode, KeyModifiers)> {
    let key_str = key_str.trim();

    // Handle modifier prefixes
    if let Some(rest) = key_str.strip_prefix("ctrl-") {
        let code = parse_key_code(rest)?;
        return Some((code, KeyModifiers::CONTROL));
    }
    if let Some(rest) = key_str.strip_prefix("alt-") {
        let code = parse_key_code(rest)?;
        return Some((code, KeyModifiers::ALT));
    }

    let code = parse_key_code(key_str)?;
    Some((code, KeyModifiers::NONE))
}

fn parse_key_code(s: &str) -> Option<KeyCode> {
    match s {
        "enter" => Some(KeyCode::Enter),
        "esc" => Some(KeyCode::Esc),
        "tab" => Some(KeyCode::Tab),
        "backtab" => Some(KeyCode::BackTab),
        "backspace" => Some(KeyCode::Backspace),
        "up" => Some(KeyCode::Up),
        "down" => Some(KeyCode::Down),
        "left" => Some(KeyCode::Left),
        "right" => Some(KeyCode::Right),
        s if s.len() == 1 => Some(KeyCode::Char(s.chars().next().unwrap())),
        _ => None,
    }
}

pub fn parse_action(action_str: &str, _mode: &Mode) -> Option<Action> {
    match action_str {
        "move_left" => Some(Action::MoveLeft),
        "move_right" => Some(Action::MoveRight),
        "move_up" => Some(Action::MoveUp),
        "move_down" => Some(Action::MoveDown),
        "move_word_forward" => Some(Action::MoveWordForward),
        "move_word_back" => Some(Action::MoveWordBack),
        "move_line_start" => Some(Action::MoveLineStart),
        "move_line_end" => Some(Action::MoveLineEnd),
        "insert_at_cursor" => Some(Action::InsertAtCursor),
        "insert_after_cursor" => Some(Action::InsertAfterCursor),
        "insert_at_line_start" => Some(Action::InsertAtLineStart),
        "insert_at_line_end" => Some(Action::InsertAtLineEnd),
        "delete_forward" => Some(Action::DeleteForward),
        "delete_back" => Some(Action::DeleteBack),
        "insert_newline" => Some(Action::InsertNewline),
        "undo" => Some(Action::Undo),
        "redo" => Some(Action::Redo),
        "delete_word_back" => Some(Action::DeleteWordBack),
        "delete_to_line_start" => Some(Action::DeleteToLineStart),
        "clear_buffer" => Some(Action::ClearBuffer),
        "clipboard_paste" => Some(Action::ClipboardPaste),
        "clipboard_copy" => Some(Action::ClipboardCopy),
        "yank" => Some(Action::Yank),
        "yank_delete" => Some(Action::YankDelete),
        "paste_register" => Some(Action::PasteRegister),
        "select_line" => Some(Action::SelectLine),
        "hover_info" => Some(Action::HoverInfo),
        "search" => Some(Action::EnterMode(Mode::Search)),
        "accept" => Some(Action::Accept),
        "abort" => Some(Action::Abort),
        "complete_start" => Some(Action::CompleteStart),
        "enter_normal" => Some(Action::EnterMode(Mode::Normal)),
        "enter_insert" => Some(Action::EnterMode(Mode::Insert)),
        "enter_visual" => Some(Action::EnterMode(Mode::Visual)),
        // Visual mode extend
        "extend_left" => Some(Action::ExtendLeft),
        "extend_right" => Some(Action::ExtendRight),
        "extend_up" => Some(Action::ExtendUp),
        "extend_down" => Some(Action::ExtendDown),
        "extend_word_forward" => Some(Action::ExtendWordForward),
        "extend_word_back" => Some(Action::ExtendWordBack),
        "extend_line_start" => Some(Action::ExtendLineStart),
        "extend_line_end" => Some(Action::ExtendLineEnd),
        // Selection actions (Helix)
        "select_left" => Some(Action::SelectLeft),
        "select_right" => Some(Action::SelectRight),
        "select_word_forward" => Some(Action::SelectWordForward),
        "select_word_back" => Some(Action::SelectWordBack),
        "delete_selection" => Some(Action::DeleteSelection),
        "change_selection" => Some(Action::ChangeSelection),
        "collapse_selection" => Some(Action::CollapseSelection),
        _ => None,
    }
}

type KeyMapEntry = ((KeyCode, KeyModifiers), Action);

pub struct ConfigKeyMap {
    normal: Vec<KeyMapEntry>,
    insert: Vec<KeyMapEntry>,
    visual: Vec<KeyMapEntry>,
}

impl ConfigKeyMap {
    pub fn from_config(config: &KeymapConfig) -> Self {
        let normal = build_entries(&config.normal, &Mode::Normal);
        let insert = build_entries(&config.insert, &Mode::Insert);
        let visual = build_entries(&config.visual, &Mode::Visual);
        Self {
            normal,
            insert,
            visual,
        }
    }
}

fn build_entries(map: &HashMap<String, String>, mode: &Mode) -> Vec<KeyMapEntry> {
    map.iter()
        .filter_map(|(action_name, key_str)| {
            let (code, modifiers) = parse_key(key_str)?;
            let action = parse_action(action_name, mode)?;
            Some(((code, modifiers), action))
        })
        .collect()
}

/// Checks if key event modifiers match the expected modifiers.
/// When expected is NONE, the key must have no modifiers (or only SHIFT for uppercase chars).
/// When expected has a modifier (e.g. CONTROL), the key must contain it.
fn modifiers_match(actual: KeyModifiers, expected: KeyModifiers) -> bool {
    if expected.is_empty() {
        actual.is_empty() || actual == KeyModifiers::SHIFT
    } else {
        actual.contains(expected)
    }
}

impl KeyMap for ConfigKeyMap {
    fn resolve(&self, mode: &Mode, key: crossterm::event::KeyEvent) -> Option<Action> {
        match mode {
            Mode::Normal => {
                for ((code, modifiers), action) in &self.normal {
                    if key.code == *code && modifiers_match(key.modifiers, *modifiers) {
                        return Some(action.clone());
                    }
                }
                None
            }
            Mode::Insert => {
                for ((code, modifiers), action) in &self.insert {
                    if key.code == *code && modifiers_match(key.modifiers, *modifiers) {
                        return Some(action.clone());
                    }
                }
                // Fallback: unbound chars in insert mode are InsertChar
                if let KeyCode::Char(ch) = key.code
                    && (key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT)
                {
                    return Some(Action::InsertChar(ch));
                }
                None
            }
            Mode::Visual => {
                for ((code, modifiers), action) in &self.visual {
                    if key.code == *code && modifiers_match(key.modifiers, *modifiers) {
                        return Some(action.clone());
                    }
                }
                None
            }
            Mode::Search => crate::editor::keymap::resolve_search(key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_with(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    // --- parse_key ---

    #[test]
    fn parse_key_simple_char() {
        let (code, mods) = parse_key("h").unwrap();
        assert!(matches!(code, KeyCode::Char('h')));
        assert_eq!(mods, KeyModifiers::NONE);
    }

    #[test]
    fn parse_key_ctrl() {
        let (code, mods) = parse_key("ctrl-r").unwrap();
        assert!(matches!(code, KeyCode::Char('r')));
        assert_eq!(mods, KeyModifiers::CONTROL);
    }

    #[test]
    fn parse_key_alt() {
        let (code, mods) = parse_key("alt-enter").unwrap();
        assert!(matches!(code, KeyCode::Enter));
        assert_eq!(mods, KeyModifiers::ALT);
    }

    #[test]
    fn parse_key_special() {
        let (code, _) = parse_key("enter").unwrap();
        assert!(matches!(code, KeyCode::Enter));
        let (code, _) = parse_key("esc").unwrap();
        assert!(matches!(code, KeyCode::Esc));
        let (code, _) = parse_key("tab").unwrap();
        assert!(matches!(code, KeyCode::Tab));
        let (code, _) = parse_key("backspace").unwrap();
        assert!(matches!(code, KeyCode::Backspace));
    }

    #[test]
    fn parse_key_uppercase() {
        let (code, mods) = parse_key("K").unwrap();
        assert!(matches!(code, KeyCode::Char('K')));
        assert_eq!(mods, KeyModifiers::NONE);
    }

    // --- parse_action ---

    #[test]
    fn parse_action_move() {
        assert!(matches!(
            parse_action("move_left", &Mode::Normal),
            Some(Action::MoveLeft)
        ));
        assert!(matches!(
            parse_action("undo", &Mode::Normal),
            Some(Action::Undo)
        ));
    }

    #[test]
    fn parse_action_search() {
        assert!(matches!(
            parse_action("search", &Mode::Normal),
            Some(Action::EnterMode(Mode::Search))
        ));
    }

    #[test]
    fn parse_action_unknown() {
        assert!(parse_action("nonexistent", &Mode::Normal).is_none());
    }

    // --- load_keymap_config ---

    #[test]
    fn load_config_from_toml() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
        write!(
            f,
            r#"
[normal]
move_left = "h"
move_right = "l"
accept = "enter"

[insert]
delete_back = "backspace"
enter_normal = "esc"
insert_newline = "enter"
"#
        )
        .unwrap();

        let content = std::fs::read_to_string(f.path()).unwrap();
        let config: KeymapConfig = toml::from_str(&content).unwrap();
        assert_eq!(config.normal.get("move_left").unwrap(), "h");
        assert_eq!(config.insert.get("delete_back").unwrap(), "backspace");
    }

    // --- ConfigKeyMap ---

    #[test]
    fn config_keymap_resolves() {
        let mut normal = HashMap::new();
        normal.insert("move_left".to_string(), "h".to_string());
        normal.insert("accept".to_string(), "enter".to_string());
        normal.insert("redo".to_string(), "ctrl-r".to_string());

        let mut insert = HashMap::new();
        insert.insert("delete_back".to_string(), "backspace".to_string());
        insert.insert("enter_normal".to_string(), "esc".to_string());

        let config = KeymapConfig {
            normal,
            insert,
            visual: HashMap::new(),
        };
        let km = ConfigKeyMap::from_config(&config);

        assert!(matches!(
            km.resolve(&Mode::Normal, key(KeyCode::Char('h'))),
            Some(Action::MoveLeft)
        ));
        assert!(matches!(
            km.resolve(&Mode::Normal, key(KeyCode::Enter)),
            Some(Action::Accept)
        ));
        assert!(matches!(
            km.resolve(
                &Mode::Normal,
                key_with(KeyCode::Char('r'), KeyModifiers::CONTROL)
            ),
            Some(Action::Redo)
        ));
        assert!(matches!(
            km.resolve(&Mode::Insert, key(KeyCode::Backspace)),
            Some(Action::DeleteBack)
        ));
    }

    #[test]
    fn config_keymap_insert_char_fallback() {
        let config = KeymapConfig {
            normal: HashMap::new(),
            insert: HashMap::new(),
            visual: HashMap::new(),
        };
        let km = ConfigKeyMap::from_config(&config);

        // Unbound chars in insert mode should still produce InsertChar
        assert!(matches!(
            km.resolve(&Mode::Insert, key(KeyCode::Char('a'))),
            Some(Action::InsertChar('a'))
        ));
    }
}
