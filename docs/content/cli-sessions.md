---
title: nk attach · write · read
group: Reference
order: 5
description: Attach to, write to and read the tmux sessions nk run creates.
---

Commands for the tmux sessions `nk run` creates.

| Command | What it does |
|---|---|
| `nk attach <session>` | Attach to a session you detached from, or one started with `--no-attach` or `--new`. |
| `nk write <session> <text>` | Paste text into the session and submit it. |
| `nk read <session>` | Print the session's current screen. `--lines N` adds scrollback. |

## nk write

Text is pasted as a bracketed paste, so a multi-line message arrives as one message in Claude Code and Codex.

```sh title="shell"
$ nk write nemonik-my-app-mitchldtn-code-review-a1b2c3d4 "review addressed, please look again"
```

:::note
Session names end in a short hash, so two runs of the same kit never share a session.
:::

:::note
An agent finds its own session name with `tmux display-message -p '#{session_name}'`.
:::
