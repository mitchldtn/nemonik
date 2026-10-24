---
title: Set up with your agent
group: Getting started
order: 3
description: Instructions for a coding agent to install and configure nemonik with you, asking before each change.
---

This page is written for coding agents. If you're a human, paste this into Claude Code or Codex:

```text title="prompt"
Read https://docs.nemonik.io/agent-setup.md and follow it to set me up with nemonik.
```

## How to work

You are setting up nemonik for the user on this machine. nemonik launches Claude Code or Codex with a kit (a system prompt, skills, MCP servers, hooks and a workflow) in a tmux session.

- Ask one question at a time, and wait for the answer.
- Before each command, say in one line what it does. Ask before any command that installs, writes a file or changes a setting.
- Skip a step that is already done, and say so.
- When you're done, summarize what you set and what the user runs next.

## 1. Check what is there

```sh title="shell"
$ nk --version
$ tmux -V
$ cat ~/.nemonik/config.json
$ nk trust
```

Tell the user what you found: whether nemonik and tmux are installed, and what `config.json` holds. A missing `config.json` is normal.

## 2. Install

Skip this if `nk --version` worked. On macOS, Homebrew installs nemonik, the `nk` alias and tmux:

```sh title="macOS"
$ brew install mitchldtn/tap/nemonik
```

On a Mac without Homebrew, the script installs `nk` to `~/.local/bin`. It doesn't install tmux; that comes from Homebrew or MacPorts (`sudo port install tmux`).

```sh title="macOS without Homebrew"
$ curl -fsSL https://raw.githubusercontent.com/mitchldtn/nemonik/main/scripts/install-macos.sh | sh
```

On Linux, the script installs `nk` to `~/.local/bin`. It doesn't install tmux; use the system's package manager for that.

```sh title="Linux"
$ curl -fsSL https://raw.githubusercontent.com/mitchldtn/nemonik/main/scripts/install-linux.sh | sh
```

If `~/.local/bin` isn't on the user's PATH, the script says so; ask before adding it to their shell profile. Run `nk --version` and `tmux -V` again to confirm.

## 3. New windows

Kits that start other kits open them in a new terminal window with `nk run --new`. For example, the sdd plan kit starts build that way. `--new` runs `new_window_command` from `~/.nemonik/config.json`, so set it now. In a herdr pane, `--new` opens a herdr tab instead and doesn't need `new_window_command` ([Use with herdr](/herdr.md)).

Find the user's terminal from `$TERM_PROGRAM` and ask them to confirm it:

| `$TERM_PROGRAM` | Terminal |
|---|---|
| `iTerm.app` | iTerm2 |
| `Apple_Terminal` | Apple Terminal |
| `WezTerm` | WezTerm |

If it is `tmux`, empty or anything else, ask which terminal they use. Take the value for it from the Window commands section of [config.md](/config.md). If their terminal isn't listed there, say so and offer a listed one they have installed.

Write it to `~/.nemonik/config.json`, keeping a `trust` value that is already there. The file accepts only `trust`, `new_window_command` and `herdr`; any other key makes every `nk` command fail.

```json title="~/.nemonik/config.json"
{
  "trust": "strict",
  "new_window_command": <the value for the user's terminal, from config.md>
}
```

Test it on a throwaway session. Fill `{session}` with `nk-setup-test` and run the command with the user's shell, `$SHELL -c <command>`, as nemonik does:

```sh title="shell"
$ tmux new-session -d -s nk-setup-test
```

If it fails with "duplicate session", an earlier run left `nk-setup-test` behind; ask, then remove it with `tmux kill-session -t nk-setup-test` and start again.

The command is one argument to `$SHELL -c`. Most window commands contain single quotes, so don't paste it inside single quotes; read it back from `config.json`, or quote it so its own quotes survive.

Ask the user whether a window opened attached to `nk-setup-test`. Then remove it:

```sh title="shell"
$ tmux kill-session -t nk-setup-test
```

### Shift+Enter

In tmux, Shift+Enter sends the message instead of starting a new line unless `extended-keys` is on. Check the user's tmux config:

```sh title="shell"
$ grep extended-keys ~/.tmux.conf ~/.config/tmux/tmux.conf
```

If it finds nothing, ask, then add the two `extended-keys` lines from the tmux settings section of [config.md](/config.md) to the file the user has, or to `~/.tmux.conf` if neither exists. Keep what is already in the file. If tmux is running, apply them with `tmux source-file` and the file's path.

## 4. Kit checks

Explain the two trust policies and ask which the user wants:

- `strict` (the default): a new or changed kit is shown in full and the user approves it before it starts.
- `off`: no check; every run prints a warning instead.

Only if they choose `off`:

```sh title="shell"
$ nk trust off
```

See [Trust and verification](/trust/).

## 5. Account

Ask whether the user will run hosted kits or use the context store. Local kits need no account. If yes:

```sh title="shell"
$ nk login
```

It opens the browser. Wait for the user to finish there, then check that the command succeeded.

## Done when

- `nk --version` and `tmux -V` work.
- `~/.nemonik/config.json` holds the chosen `new_window_command` and, if changed, `trust`, and `nk trust` prints the chosen policy.
- The test window opened.
- `nk login` succeeded, if the user wanted an account.

:::more
- [Quickstart](/quickstart/): The same steps by hand.
- [config.json](/config/): Every setting and the window commands.
:::
