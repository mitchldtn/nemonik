---
title: nk attach · write · read
group: Reference
order: 5
description: Attach to, write to and read the tmux sessions nk run creates.
---

Commands for the tmux sessions `nk run` creates.

| Command | What it does |
|---|---|
| `nk attach <session>` | Attach to a session you detached from, or one started with `--no-attach` or `--new`. In a herdr pane, herdr also shows the session's agent ([Use with herdr](/herdr/)). |
| `nk write <session> <text>` | Paste text into the session and submit it. |
| `nk read <session>` | Print the session's current screen. `--lines N` adds scrollback. |

## nk write

Text is pasted as a bracketed paste, so a multi-line message arrives as one message in Claude Code and Codex.

```sh title="shell"
$ nk write code-review-a1b2c3 "review addressed, please look again"
```

:::note
Session names are the kit's category and name plus 6 random hex characters, like `code-review-a1b2c3`, so two runs of the same kit never share a session. `nk run --name` sets your own.
:::

:::note
An agent finds its own session name with `tmux display-message -p '#{session_name}'`.
:::
