---
title: The context store
group: Context
order: 1
description: Docs your agent reads and writes over MCP; owned and referenced docs, the well-known docs, and the session brief.
---

Docs your agent reads and writes over MCP, so what a project learned survives from one session to the next.

## Owned and referenced

- **Owned.** Stored in full with `put`. A doc is identified by its key, set from a `slug`, not by a file path or ticket.
- **Referenced.** A PR or ticket is stored as a pointer only, never its body. Create tickets in the tracker itself.

## Well-known docs

| Doc | What it holds |
|---|---|
| `status` :tag[one per project] | Where the project is now: what shipped, what's in flight. |
| `memory` :tag[one per project] | How the team works; context that outlives a task. |
| `journal` | What happened and what was learned, added to with `mode="append"`. |
| `plan` | A plan for a piece of work, by slug. |

## The session brief

`brief` returns a project's starting context in one call: `status` and `memory`, plus a journal and a plan by slug when you name them.

```mcp title="MCP"
brief(project="my-app")
brief(project="my-app", journal="dev-131-journal", plan="dev-131-plan")
```

:::note
Every doc has a free-form `type` label; the default is `code_task`.
:::
