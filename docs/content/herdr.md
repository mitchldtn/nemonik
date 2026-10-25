---
title: Use with herdr
group: Guides
order: 3
description: How nk run and nk attach behave in a herdr pane, with agent status, restore after a restart, sessions in tabs, and the off switch.
---

[herdr](https://herdr.dev) is a terminal workspace manager for coding agents. nemonik sessions still run in tmux. In a herdr pane, `nk run` and `nk attach` also tell herdr about them.

## In a herdr pane

- herdr shows the session's agent, Claude Code or Codex, and its status: working, blocked or idle.
- After herdr restarts, the pane runs `nemonik attach <session>` on its own and shows the same session. This needs herdr 0.9.2 or newer.
- The pane's herdr agent is named after the session, so `herdr agent wait <session>` and `herdr agent prompt <session>` work. If another pane already shows the session, the second pane's agent has no name.
- `nk run --new` opens the session in a new herdr tab labeled with the kit and switches to it. It doesn't use `new_window_command`. `--no-attach` opens the same tab in the background.
- The tab closes when the session ends or you detach. After a herdr restart, a restored pane goes back to a shell instead.
- A kit started from inside such a session, like sdd plan starting build, opens as another tab in the same herdr workspace.

```sh title="shell"
$ nk run --new --name review-128 mitchldtn/code:review "review pr 128"
$ herdr agent wait review-128 --until idle
```

## Workspaces and worktrees

- Open one herdr workspace per project folder: `herdr workspace create --cwd <folder>`, or new in the sidebar. `nk run` there finds the folder's `nemonik.json`, so its sessions use that project and its aliases.
- `nk run --new` and `--no-attach` open their tab in the workspace of the pane they run in, even when another workspace is focused.
- `herdr worktree create --branch <name>` opens a Git worktree as its own workspace, grouped under the repository's workspace. The checkout has `nemonik.json` only if it is committed. `herdr worktree remove` removes the checkout and keeps the branch.
- Closing a workspace closes its tabs. Its nemonik sessions keep running; reach them with `nk attach <session>`.

## Notifications

herdr shows notifications only when `delivery` under `[ui.toast]` is set in herdr's `config.toml`. It is `off` by default.

```toml title="~/.config/herdr/config.toml"
[ui.toast]
delivery = "terminal"
```

- `terminal`: the terminal shows them. Use it in iTerm2, Ghostty, WezTerm and kitty.
- `system`: the operating system shows them. On macOS, install `terminal-notifier` (`brew install terminal-notifier`) so clicking one opens the terminal.

herdr shows none for the tab you are on. Apply an edited config with `herdr server reload-config`.

## Sessions started outside herdr

`nk attach <session>` in a herdr pane shows a session started outside herdr the same way. Sessions started by an older nemonik attach without a status.

A session started outside herdr starts kits with `--new` in a new window, using `new_window_command`.

Run `nk attach` from a herdr pane, not from inside tmux. Inside tmux it switches the tmux client, and herdr isn't told.

## The ctrl+b prefix

herdr and tmux both use `ctrl+b` as their prefix key, and herdr gets it first. Change one of them, for example tmux's:

```sh title="~/.tmux.conf"
set -g prefix C-a
unbind C-b
bind C-a send-prefix
```

Or herdr's, with `prefix` under `[keys]` in herdr's `config.toml`.

## Turn it off

Set `"herdr": false` in [config.json](/config/). nemonik then behaves in herdr as it does outside it, and `--new` uses `new_window_command`.
