---
title: nk login
group: Reference
order: 1
description: Log in with OAuth in the browser; tokens in the OS keychain, or NEMONIK_API_KEY for headless use.
---

Logs you in with an OAuth flow in your browser and stores the tokens in your OS keychain.

```text title="usage"
nk login
```

## Tokens

Tokens are stored under the keychain service `io.nemonik.cli`. Access tokens are short-lived and refresh on their own; log in again only if the refresh token expires from long disuse.

## Headless and CI

Where there is no browser or keychain, set `NEMONIK_API_KEY` to a nemonik API key (`nmk_...`). It is checked before the keychain on every command.

tmux sessions get the environment tmux started with. Put the export in your shell profile, or add this to `~/.tmux.conf`:

```text title="~/.tmux.conf"
set -ga update-environment 'NEMONIK_API_KEY'
```

If tmux was already running when you set the key, run `tmux set-environment -g NEMONIK_API_KEY "$NEMONIK_API_KEY"` or restart tmux.

:::note
The flow is CIMD with PKCE, the same one nemonik's MCP connector uses.
:::

:::note
tmux older than 3.0 needs a leading space: `' NEMONIK_API_KEY'`.
:::

:::note
`~/.nemonik/auth.lock` holds no secret and is safe to delete.
:::
