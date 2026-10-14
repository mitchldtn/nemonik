---
title: put · get · search
group: Context
order: 2
description: The put, get and search MCP tools for writing, reading and finding context docs.
---

Three MCP tools: `put` writes, `get` reads one doc, `search` finds many.

## put

`put` replaces the whole body by default; `mode="append"` adds a section instead. Pass the same `slug` to update a doc in place.

```mcp title="MCP"
put(project="my-app", slug="architecture", body="...")
put(project="my-app", slug="dev-52-journal", mode="append", body="...")
put(project="my-app", slug="architecture", links=["..."])  # metadata only
```

## get

```mcp title="MCP"
get(type="context", project="my-app", slug="architecture")
get(type="context", project="my-app", ticket="DEV-52")
```

## search

Ranked by relevance. `since` and `until` take ISO-8601 times; with a range and no query, results come back newest first.

```mcp title="MCP"
search(query="trust gate", project="my-app", limit=8)
search(since="2026-10-01T00:00:00Z")
```

:::note
A `ticket` alone doesn't update in place: a reworded title makes a second doc. Use `slug`.
:::

:::note
Omitting `links` on a replace clears them. Pass them again.
:::
