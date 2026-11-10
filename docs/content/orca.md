---
title: Use with Orca
group: Guides
order: 4
description: How nk run and nk attach behave in an Orca terminal, with agent status, sessions in terminals of the same worktree, Codex accounts and the off switch.
---

[Orca](https://www.onorca.dev) is a desktop app for running coding agents in parallel worktrees. nemonik sessions still run in tmux. In an Orca terminal, `nk run` also tells Orca about them. Tested with Orca 1.4.255 on macOS, Claude Code 2.1.308 and Codex 0.164.0.

## In an Orca terminal

- Orca shows the session's agent, Claude Code or Codex, and its status on the terminal that shows the session: working, waiting for a permission and done. Orca's agent hooks must be on (`orca agent hooks status`).
- `nk run --new` opens the session in a new Orca terminal in the same worktree and switches to it. It doesn't use `new_window_command`. `--no-attach` opens the same terminal in the background.
- The new terminal's title shows the session and the kit, such as `sdd-plan-a1b2c3 - mitchldtn/sdd:plan`.
- The terminal closes when the session ends or you detach.
- A kit started from inside such a session, like sdd plan starting build, opens as another terminal in the worktree the session was started in, even when another worktree is focused.

```sh title="shell"
$ nk run --new mitchldtn/sdd:plan "add recurring tasks"
```

If Orca can't open the terminal, the session keeps running and `nk run` prints `nemonik attach <session>`.

## Codex

- A Codex session uses the Codex home of the terminal it was started from. With a Codex account selected in Orca, it runs as that account.
- In Orca, nemonik starts Codex with `--no-daemon`, as Orca does for the `codex` you type in its terminals.

## Permissions

By default, Orca starts the agents it launches itself without permission prompts. A kit started with `nk run` keeps the agent's own prompts, and Orca shows the session as waiting while one is open. `nk run --yolo` skips them.

## Status stays with the first terminal

A session reports its status to the terminal it first showed in. `nk attach <session>` in another Orca terminal shows the session, but its status stays on the first terminal, or shows nowhere once that terminal is closed.

A session started outside Orca, for example in iTerm2, shows no status in Orca, and its kits open with `--new` in a new window, using `new_window_command`.

## Not covered

- Kits in Orca's agent picker or orchestrator.
- Orca on Linux or Windows, remote Orca hosts and SSH workspaces: untested.

## herdr in Orca

In a herdr pane inside an Orca terminal, nemonik behaves as in herdr ([Use with herdr](/herdr/)).

## Turn it off

Set `"orca": false` in [config.json](/config/). nemonik then behaves in Orca as it does outside it, and `--new` uses `new_window_command`.
