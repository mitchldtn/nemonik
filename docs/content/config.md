---
title: config.json
group: Reference
order: 8
description: Your settings in ~/.nemonik/config.json, trust and new_window_command, with window commands per terminal and tmux settings.
---

Your settings, in `~/.nemonik/config.json`. Both keys are optional.

```json title="~/.nemonik/config.json"
{
  "trust": "strict",
  "new_window_command": "gnome-terminal -- tmux attach -t {session}"
}
```

| Key | What it does |
|---|---|
| `trust` | `strict` or `off`. Written by `nk trust`. |
| `new_window_command` | The command `run --new` runs, with a `{session}` placeholder. |

## Window commands

Pick the one for your terminal and copy it, quotes included, as the value of `new_window_command`. The backslashes are JSON escapes. Terminals that stay in the foreground need `&`, because nemonik waits for the command to finish.

```sh title="macOS"
# iTerm2
"osascript -e 'tell application \"iTerm2\" to create window with default profile command \"/bin/zsh -lc \\\"tmux attach -t {session}\\\"\"'"

# Apple Terminal
"osascript -e 'tell application \"Terminal\" to do script \"tmux attach -t {session}\"'"

# WezTerm
"wezterm cli spawn --new-window -- tmux attach -t {session}"
```

```sh title="Linux"
# GNOME Terminal
"gnome-terminal -- tmux attach -t {session}"

# WezTerm
"wezterm cli spawn --new-window -- tmux attach -t {session}"

# Konsole
"konsole -e tmux attach -t {session} &"

# xterm
"xterm -e tmux attach -t {session} &"

# Alacritty
"alacritty -e tmux attach -t {session} &"

# kitty
"kitty tmux attach -t {session} &"
```

nemonik runs the command with `$SHELL -c`. When you write your own:

- Don't put `=` before `{session}`. In zsh, `=name` expands to the path of a command, so the window closes at once.
- If the terminal doesn't start a login shell (iTerm2's command doesn't), `tmux` may not be on PATH. Wrap the inner command in `/bin/zsh -lc`, as the iTerm2 line does.

### tmux settings

Without `extended-keys`, Shift+Enter in a tmux window sends the message instead of starting a new line. Add these to `~/.tmux.conf` (tmux 3.2 or newer), then run `tmux source-file ~/.tmux.conf`.

```sh title="~/.tmux.conf"
set -g mouse on

# Pass Shift+Enter (and other modified keys) through to apps like Claude Code
set -s extended-keys on
set -as terminal-features 'xterm*:extkeys'
```

:::note
Unknown keys are an error, so a misspelled key is reported instead of ignored. `nk trust` writes the file `0600`.
:::

:::note
The file is plain JSON. Inner quotes in a command need escaping with `\"`.
:::
