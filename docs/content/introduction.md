---
title: Introduction
group: Getting started
order: 1
description: What nemonik is, why kits, and the pieces it is built from.
---

nemonik packages how your team works into a kit: a system prompt, skills, MCP servers, hooks and a workflow. The workflow says what to do and which skills to use. nemonik injects it into the system prompt, the first message or both, so the agent follows it. Running a kit starts Claude Code or Codex with all of it in place, so planning, coding and reviewing are each one command. Kits can also hand off to each other, chaining those steps so each one runs in its own session with a fresh context.

## Why nemonik

Most agent setups grow in one direction: a `CLAUDE.md` that collects every rule, a folder of skills the agent may or may not pick up, and every MCP server switched on for every task. The agent reads all of it in every session, and follows less of it as it grows.

A kit holds one way of working, so the agent only gets what that step needs. System prompts can embed other prompts, so kits are built from shared pieces instead of copies, and your team runs the same kit you do.

A running memory, journal and status carry what each session learned into the next.

## The pieces

- **Kits.** A system prompt, skills, MCP servers and hooks, tied together by a workflow. System prompts can embed other prompts. See [What is a kit?](/what-is-a-kit/)
- **Workflows.** Say which skills to use and when. Kits chain by handing off to each other. See [What is a kit?](/what-is-a-kit/) and [a coder and a reviewer](/local-kits/#example-a-coder-and-a-reviewer).
- **Memory.** A running memory, journal and status your agent reads and writes over MCP. See [The context store](/context-store/).

:::more
- [Quickstart](/quickstart/): Install to a running agent in a few commands.
- [What is a kit?](/what-is-a-kit/): The bundle nemonik resolves and launches.
:::

:::note
`nk` and `nemonik` are the same command. These docs use `nk`.
:::
