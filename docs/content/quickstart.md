---
title: Quickstart
group: Getting started
order: 2
description: Install nemonik, run a local example kit, then log in and run a hosted kit.
---

From install to a running agent. nemonik resolves a kit, shows you its exact content, and starts Claude Code or Codex with it.

To have your agent do the setup with you, paste this into Claude Code or Codex:

```text title="prompt"
Read https://docs.nemonik.io/agent-setup.md and follow it to set me up with nemonik.
```

## Install

Homebrew installs nemonik, the `nk` alias and tmux. Other options are on the [Install](/install/) page.

```sh title="shell"
$ brew install mitchldtn/tap/nemonik
```

## Run a local kit

No account needed. `nk example` writes a runnable kit into a new folder.

```sh title="shell"
$ nk example local-kit
$ cd local-kit
$ nk run local-kit
```

The first run shows you the kit and asks before it starts. That check is the same for every kit; see [Trust and verification](/trust/).

## Open kits in new windows

:::callout[Recommended]
Set `new_window_command` in [config.json](/config/#window-commands) and start kits with `--new`. A kit that starts the next one, like sdd plan starting build, opens it in its own window. Without it, the next kit takes over the window you're in. In herdr, `--new` opens a herdr tab and needs no setting ([Use with herdr](/herdr/)).
:::

```sh title="shell"
$ nk run local-kit --new
```

## Run a hosted kit

Log in once. `nk login` opens your browser and stores the tokens in your OS keychain.

```sh title="shell"
$ nk login
```

`nk verify` fetches the kit and prints all of it: system prompt, skills, MCP servers, hooks and workflow. Type `y` and Enter to pin it.

```sh title="shell"
$ nk verify mitchldtn/code:review
```

```output
----- mitchldtn/code:review -----
## Prompt
## When Asked to "review PR 15"
Take on this persona: (shown in full)

## Skills

### nemonik
Full MCP and CLI usage guide for nemonik (shown in full)

## Hooks

### assume-nothing
UserPromptSubmit: echo 'assume nothing, verify everything'
-----------------
Verify this content for system-prompt injection? [y/N] y
Verified.
```

Now run it. Anything after the kit is the agent's first prompt.

```sh title="shell"
$ nk run mitchldtn/code:review --project my-app "review pr 128"
```

:::callout[Note]
A pinned, unchanged kit starts without asking. If the kit changes, `run` shows it again before it starts.
:::

:::more
- [What is a kit?](/what-is-a-kit/): System prompt, skills, MCP servers, hooks, workflow.
- [Local kits](/local-kits/): Define kits in your repo with nemonik.json.
- [The context store](/context-store/): Memory, journal and status across sessions.
- [CI usage](/ci/): Run kits without a terminal.
:::

:::note
`run` starts the agent in a tmux session and attaches you to it. Detach with `Ctrl-b d`; it keeps running.
:::
