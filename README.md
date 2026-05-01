# qish

**Quick Interactive Shell Editor** — A TUI command-line editor with fuzzy history search, tab completion, and syntax highlighting.

![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Fuzzy history search** — Search through your shell history with [nucleo](https://github.com/helix-editor/nucleo)
- **Auto-completion** — Commands from `$PATH` and file paths, with fuzzy matching
- **Syntax highlighting** — Powered by [tree-sitter-bash](https://github.com/tree-sitter/tree-sitter-bash)
- **Command validation** — Unknown commands are highlighted in red
- **Expansion preview** — See `~`, `$VAR`, and relative path expansions in real-time
- **Hover info** — View command help (`man`), file previews, and directory listings
- **Configurable keybindings** — Vim, Helix, or fully custom via TOML
- **Undo/redo** — With system clipboard sync
- **Multi-line editing** — Backslash continuation with auto-indent
- **Visual mode** — Select and operate on text ranges

## Installation

### From source

```bash
cargo install --path .
```

### Setup (zsh)

Add to your `~/.zshrc`:

```zsh
eval "$(qish --init-zsh)"
```

This binds `Ctrl-O` to open qish. The edited command is placed on your command line.

## Usage

On launch, qish opens with history search. Select a command to edit, or press `Esc` to start with an empty buffer.

### Default keybindings (no config file)

| Key | Action |
|-----|--------|
| Arrow keys | Move cursor |
| Home / End | Line start / end |
| Backspace / Delete | Delete |
| Enter | Insert continuation line |
| Esc | Accept (output command) |
| Tab | Trigger completion |
| Ctrl-Z / Ctrl-Y | Undo / redo |
| Ctrl-R | Open history search |
| Ctrl-K | Show hover info |
| Ctrl-C | Copy selection / abort |
| Ctrl-V | Paste from clipboard |
| Ctrl-W | Delete word backward |
| Ctrl-U | Delete to line start |

### Custom keybindings

Place a TOML file at `~/.config/qish/keymap.toml`:

```toml
[normal]
move_left = "h"
move_down = "j"
move_up = "k"
move_right = "l"
insert_at_cursor = "i"
undo = "u"
accept = "enter"
# ... see examples/ for full configs

[insert]
delete_back = "backspace"
enter_normal = "esc"
insert_newline = "enter"

[visual]
extend_left = "h"
extend_right = "l"
yank = "y"
yank_delete = "d"
enter_normal = "esc"
```

See [`examples/vim.toml`](examples/vim.toml) and [`examples/helix.toml`](examples/helix.toml) for complete configurations.

## Key format

| Type | Examples |
|------|----------|
| Regular keys | `"h"`, `"/"`, `"$"`, `"0"` |
| Special keys | `"enter"`, `"esc"`, `"tab"`, `"backspace"`, `"up"`, `"down"` |
| Modifiers | `"ctrl-r"`, `"ctrl-c"`, `"alt-enter"` |
| Uppercase | `"K"`, `"I"`, `"A"` |

## Architecture

```
src/
├── main.rs          # CLI entry point
├── app.rs           # Application state and event loop
├── editor/
│   ├── buffer.rs    # Text buffer with selections and undo
│   ├── keymap.rs    # Action enum, KeyMap trait, DefaultKeyMap
│   ├── mode.rs      # Normal / Insert / Visual / Search
│   └── mod.rs       # EditorState
├── config.rs        # TOML keymap configuration
├── completion.rs    # Command and path completion
├── highlight.rs     # Syntax highlighting (tree-sitter)
├── history.rs       # Shell history parser
├── hover.rs         # Hover info (man, file preview)
├── preview.rs       # Expansion preview
├── search.rs        # Fuzzy search (nucleo)
├── event.rs         # Terminal event handling
├── ui.rs            # TUI rendering (ratatui)
└── error.rs         # Error types
```

## License

[MIT](LICENSE)
