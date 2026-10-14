---
title: What is a kit?
group: Kits
order: 1
description: The parts of a kit (system prompt, skills, MCP servers, hooks, workflow) and its settings.
---

A kit is what `nk run` resolves, checks and starts: a system prompt, skills, MCP servers, hooks and a workflow, with a few settings.

## The parts

Every part is optional.

- **System prompt.** Text added to the agent's system prompt, for example a code-review template.
- **Skills.** Reusable guides, written to disk as `SKILL.md` files for the length of a session.
- **MCP servers.** Server registrations, a `stdio` command or an `http` URL. One registration can expose many tools.
- **Hooks.** A Claude Code hooks settings fragment, passed with `--settings`.
- **Workflow.** Says what to do and which skills to use. nemonik injects it into the system prompt, the first message or both (set by `workflow_inject`), so the agent follows it instead of guessing when to use a skill.

## Settings

| Setting | What it does |
|---|---|
| `mcp_mode` | `extend` (default) adds the kit's MCP servers to the agent's own. `only` replaces them. |
| `launch_args` | Extra flags appended to the agent's command line. |
| `workflow_inject` | `system` (the system prompt), `user` (the first message) or `both` (the default). |

:::callout[Example]
The `mitchldtn/code:review` kit is a system prompt, one skill (`nemonik`) and one hook (`assume-nothing`), with no MCP servers or workflow.
:::

:::more
- [Authoring](/authoring/): Create and edit kits over MCP.
- [Kit names and nemonik.json](/kit-names/): How a kit is named, and repo defaults.
:::

:::note
Everything in a kit, settings included, is part of the pinned hash. Changing any of it asks again.
:::
