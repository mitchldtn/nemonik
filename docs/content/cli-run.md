---
title: nk run
group: Reference
order: 3
description: Start Claude Code or Codex in a tmux session with a kit loaded; arguments, options and examples.
---

Starts Claude Code or Codex in a tmux session with a kit loaded.

```text title="usage"
nk run [OPTIONS] <KIT> [PROMPT]...
```

## Arguments

| Argument | What it is |
|---|---|
| `KIT` | A full kit name (`owner/category:name`), or an alias or local kit name from `nemonik.json`. |
| `PROMPT` | The agent's first task. Put it in double quotes so your shell passes it through unchanged, including characters like `?`, `'`, `<` and `>`. |

## Options

| Option | What it does |
|---|---|
| `--project` | Project for `--remote` access and the session name. Defaults to `$NEMONIK_PROJECT`, then `nemonik.json`. |
| `--agent` | `claude` (default) or `codex`. |
| `--no-attach` | Start the session in the background and print how to attach. |
| `--new` | Open the session in a new terminal window, using `new_window_command` from [config.json](/config/). |
| `--remote` | Let your account write to or watch the session from another device. Needs `--project`. |
| `--yes` | Start a new or changed kit without review and without pinning, for this run only. |
| `--trust` | `strict` or `off` for this run only. |
| `--yolo` | Skip the agent's own permission prompts (`--dangerously-skip-permissions` on Claude, `--dangerously-bypass-approvals-and-sandbox` on Codex). |

## The session

`run` starts the agent in a named tmux session and attaches you to it. Detach with `Ctrl-b d`; come back with [nk attach](/cli-sessions/). Every run gets its own session, even for the same kit.

## Skills and workflow

A kit's skills are written to `~/.nemonik/sessions/<session>/` and given to the agent, never into your working directory. They are removed when the session ends; for a session that died first, `nk cleanup-skills <session>` removes them.

Skills are available to the agent but not invoked on their own. The kit's workflow says which ones to use. nemonik injects it into the system prompt, the first message or both. Without a workflow, the agent starts with only your task.

## Examples

```sh title="shell"
# review a PR in a new window
$ nk run mitchldtn/code:review --project my-app --new "review pr 128"

# the same kit on Codex
$ nk run mitchldtn/code:review --agent codex

# leave it running and drive it from another device
$ nk run mitchldtn/code:review --project my-app --remote --no-attach
```

:::note
Claude is always started with `--verbose`, so its transcript shows exactly what was sent as the system prompt.
:::

:::note
On Codex, hooks aren't applied yet, `mcp_mode: only` can't be enforced, and an http MCP's headers and env are dropped; nemonik warns and continues.
:::

:::note
On Codex, a kit with skills gets its own `CODEX_HOME` under `~/.nemonik/sessions/<session>/`. It links `auth.json` and `skills/.system` from your Codex home (`$CODEX_HOME`, else `~/.codex`) and copies `config.toml`. A kit started from inside such a session links the same files and copies that session's `config.toml`. If Codex keeps your login in the keyring (`cli_auth_credentials_store` set to `keyring` or `auto`), the session starts logged out; use the default, `file`. Nothing else from it is carried in, such as `AGENTS.md`, and the session doesn't appear in `codex resume`.
:::
