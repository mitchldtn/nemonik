---
title: put · get · search
group: Context
order: 2
description: The put, get and search MCP tools for writing, reading and finding context docs.
---

Three MCP tools: `put` writes, `get` reads one doc, `search` finds many.

## put

`put` replaces the whole body by default; `mode="append"` adds a section instead. Pass the same `slug` to update a doc in place.

`mode="edit"` changes one passage of an existing doc: `old_str` must match exactly once, byte for byte, and is replaced with `new_str`; the rest of the doc stays as it is. The edit is refused, and nothing changes, when `old_str` is missing, empty or appears more than once, when no doc has that `slug`, or when `body` is passed too.

`edits` changes several passages in one write: with `mode="edit"`, pass a list of `old_str`/`new_str` pairs in place of `old_str` and `new_str`. The edits apply in order, each to the doc as the edits before it left it; an empty list changes nothing. If any edit is refused, none are applied, and the error names the edit by its index from 0 (`edit 1: old_str not found in 'dev-52-tasks'`). Passing `edits` with `old_str`, `new_str` or `body`, or without `mode="edit"`, is refused.

```mcp title="MCP"
put(project="my-app", slug="architecture", body="...")
put(project="my-app", slug="dev-52-journal", mode="append", body="...")
put(project="my-app", slug="status", mode="edit", old_str="- DEV-52 in review", new_str="- DEV-52 shipped")
put(project="my-app", slug="dev-52-tasks", mode="edit", edits=[{"old_str": "- [ ] 2.1", "new_str": "- [x] 2.1"}, {"old_str": "- [ ] 2.2", "new_str": "- [x] 2.2"}])
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
`links` only adds: a `put` without `links` keeps the doc's existing links.
:::
