---
title: CI usage
group: Guides
order: 2
description: Run kits where there is no browser, keychain or terminal, with NEMONIK_API_KEY, --yes and --no-attach.
---

Running kits where there is no browser, keychain or terminal.

## Auth

Set `NEMONIK_API_KEY`. It is checked before the keychain, so no browser login is needed.

## Getting past the check

Under `strict`, a new or changed kit stops when there is no terminal. Either pin it first on a machine with a terminal, or pass `--yes` to start it without review and without pinning. `--yes` lasts one run.

## Starting

```sh title="shell"
$ nk run mitchldtn/code:review --yes --yolo --no-attach "review pr 128"
```

`--no-attach` returns to the shell instead of attaching to the session.
## Permissions

`--yolo` skips the agent's own permission prompts. It controls what the agent may do once running, not whether nemonik trusts the kit. Use it only in a sandbox with nothing sensitive reachable.

:::note
The runner still needs tmux installed.
:::
