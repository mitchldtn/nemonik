---
title: nk remote · watch
group: Reference
order: 6
description: Write to or watch your own running session from another device through nemonik's server.
---

Remote control is off unless you start it for a session. Once it's on, you can write to or watch your own running session from another device through nemonik's server, without network access to the machine running it.

```text title="usage"
nk remote --project <PROJECT> <SESSION>
nk watch --project <PROJECT> <SESSION>
```

## Starting it

`nk run --remote` starts a second tmux session, `<session>-daemon`, that holds the connection to the server. `nk remote` starts it for a session that is already running, or restarts it if it died. The daemon exits on its own once the agent session is gone; to stop it sooner, run `tmux kill-session -t <session>-daemon`.

## Watching

`nk watch` streams a session's live output.

## Who can control it

Only the account that started remote control for the session. Other project members, the project's owner included, get "not currently connected". Every command the session receives is appended to `~/.nemonik/harness-daemon.log` on the machine running it.

:::note
This is remote code execution over an authenticated channel.
:::
