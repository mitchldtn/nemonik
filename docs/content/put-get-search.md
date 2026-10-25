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

`mode="restore"` makes an earlier version's body the doc's next version, byte for byte; the title, type and links stay. It takes only `slug` and `version`. Restoring a body equal to the current one changes nothing. Like every `put`, it needs editor rank on the project.

```mcp title="MCP"
put(project="my-app", slug="status", mode="restore", version=12)
```

## get

Every read returns the doc's current `version`. A write that changes the body, or any append, raises it by 1; a write that changes only metadata doesn't.

```mcp title="MCP"
get(type="context", project="my-app", slug="architecture")
get(type="context", project="my-app", ticket="DEV-52")
```

With a `slug`, `version=N` returns version N in the same shape: its body, title and type as they were.

`history=true` lists the doc's versions newest first, without bodies: `version`, `title`, `type`, `author` (an email) and `created_at`. `limit` sets how many come back (1 to 250, default 25); `before=N` returns only versions below N, so pass the last `version` you got to read the next page. `author` and `created_at` are null where nemonik's server has no record of the write, such as for version 1.

```mcp title="MCP"
get(type="context", project="my-app", slug="status", version=12)
get(type="context", project="my-app", slug="status", history=true)
get(type="context", project="my-app", slug="status", history=true, limit=25, before=31)
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
