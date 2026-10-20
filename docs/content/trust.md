---
title: Trust and verification
group: Getting started
order: 5
description: How nk verify pins a kit's hash, how nk run checks it, and the trust policy.
---

Nothing from nemonik reaches an agent's system prompt until a person has looked at that exact content, unless you choose to skip the check.

## verify pins a hash

`nk verify` shows a kit's exact content and asks `[y/N]`. Type `y` and Enter to approve. nemonik then saves the content's SHA-256 hash and the kit's id in `~/.nemonik/verified.json`.

`verify --web` also opens the same content in your browser, with a diff when it changed. The question in the terminal is always the one that counts. MCP env and header values are masked, and the page is written to `~/.nemonik/last-diff.html` (`0600`, overwritten each time).

## run checks it

Under the default `strict` policy, `nk run` hashes the kit again before it starts:

| Situation | What happens |
|---|---|
| Pinned and unchanged | Starts without asking. |
| New or changed, on a terminal | Shows the content and asks `[y/N]`. Yes pins and starts; no stops. |
| New or changed, no terminal | **Stops.** Nothing starts. Verify it first, or pass `--yes`. |

A kit also counts as changed when its name now points to a different kit, even if the content is identical.

## Policy and overrides

`nk trust strict|off` sets the policy, saved in [config.json](/config/). The first of these that is set wins: the `--trust` flag, the `NEMONIK_TRUST` environment variable, `config.json`, then `strict`.

A repository's `nemonik.json` can never change the policy, so a cloned repo can't switch the check off for itself.

:::note
Secrets in a kit's MCP settings are removed before the copy is saved to `verified.json`, and the file is written `0600`.
:::
