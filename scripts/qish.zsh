# qish shell integration for zsh
#
# Setup:
#   1. Install qish:
#      cargo install --path /path/to/qish
#
#   2. Add to your ~/.zshrc:
#      eval "$(qish --init-zsh)"
#
#      Or source this file directly:
#      source /path/to/qish/scripts/qish.zsh
#
# Usage:
#   Press Ctrl-O to open qish. Edit the command, then press Esc (or Enter
#   in Normal mode) to accept. The edited command is placed on your
#   command line, ready to execute with Enter.
#
# Customization:
#   To change the keybinding, replace '^O' below:
#     bindkey '^X^E' qish-edit    # Ctrl-X Ctrl-E
#     bindkey '^T' qish-edit      # Ctrl-T

function qish-edit() {
    local result
    result=$(qish)
    if [[ $? -eq 0 && -n "$result" ]]; then
        LBUFFER="$result"
    fi
    zle redisplay
}
zle -N qish-edit
bindkey '^O' qish-edit
