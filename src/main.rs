mod app;
mod completion;
mod config;
mod editor;
mod error;
mod event;
mod highlight;
mod history;
mod hover;
mod preview;
mod search;
mod ui;

use std::env;
use std::process;

use app::App;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
qish - Quick Interactive Shell Editor

USAGE:
    qish [OPTIONS]

OPTIONS:
    -h, --help       Show this help message
    -v, --version    Show version

DESCRIPTION:
    A TUI command-line editor with fuzzy history search, tab completion,
    syntax highlighting, and configurable keybindings.

    On launch, history search opens automatically. Select a command to
    edit, or press Esc to start with an empty buffer.

    The edited command is printed to stdout on accept.

DEFAULT KEYBINDINGS:
    Arrow keys       Move cursor
    Home / End       Line start / end
    Backspace        Delete backward
    Delete           Delete forward
    Enter            Insert continuation line (backslash + newline)
    Esc              Accept (output command to stdout)
    Tab              Trigger completion
    Ctrl-Z           Undo
    Ctrl-Y           Redo
    Ctrl-R           Open history search
    Ctrl-K           Show hover info (command help / file preview)
    Ctrl-C           Copy selection / abort if no selection
    Ctrl-V           Paste from clipboard
    Ctrl-W           Delete word backward
    Ctrl-U           Delete to line start
    Ctrl-L           Clear buffer
    Ctrl-A           Move to line start
    Ctrl-E           Move to line end

CONFIGURATION:
    Place a keymap file at ~/.config/qish/keymap.toml
    See examples/vim.toml and examples/helix.toml for reference.

SHELL INTEGRATION (zsh):
    eval \"$(qish --init-zsh)\"
    # or source the script from scripts/qish.zsh
";

const ZSH_INIT: &str = "\
# qish shell integration for zsh
function qish-edit() {
    local result
    result=$(qish)
    if [[ $? -eq 0 && -n \"$result\" ]]; then
        LBUFFER=\"$result\"
    fi
    zle redisplay
}
zle -N qish-edit
bindkey '^O' qish-edit
";

fn main() {
    if let Some(arg) = env::args().nth(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return;
            }
            "-v" | "--version" => {
                println!("qish {VERSION}");
                return;
            }
            "--init-zsh" => {
                print!("{ZSH_INIT}");
                return;
            }
            other => {
                eprintln!("qish: unknown option: {other}");
                eprintln!("Try 'qish --help' for more information.");
                process::exit(1);
            }
        }
    }

    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();

    match result {
        Ok(Some(cmd)) => print!("{cmd}"),
        Ok(None) => process::exit(1),
        Err(e) => {
            eprintln!("qish: {e}");
            process::exit(1);
        }
    }
}
