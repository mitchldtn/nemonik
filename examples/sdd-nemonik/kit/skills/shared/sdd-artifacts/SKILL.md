---
name: sdd-artifacts
description: How to read and write sdd's nemonik context documents, and the list of documents with their slugs and types. Use before any store call.
---

# sdd artifacts (nemonik store)

## Reading and writing

Every call takes `project=<key>`, the `project` field of `nemonik.json`.

- **Read one document:** `get(type="context", project=<key>, slug="<slug>")`. A "no context found"
  error means the document doesn't exist yet; it isn't a failure.
- **List the specs:** `search(project=<key>, type="spec", since="2000-01-01T00:00:00Z", limit=100)`.
- **Change part of a document:** `put(project=<key>, slug="<slug>", mode="edit",
  old_str="<exact current text>", new_str="<new text>")`. `old_str` must appear exactly once; include
  enough of the line to make it unique. Use this for a single change: a State line, the Base line.
- **Change several parts at once:** `put(project=<key>, slug="<slug>", mode="edit",
  edits=[{"old_str": "...", "new_str": "..."}, ...])`. The edits apply in order, each to the text the
  earlier ones left; if one is refused, none land and the error names its index. Write each document
  once per milestone: one `edits` put for all its changes (ticks, results, status lines), or one full
  `put` right after a `get`.
- **Write or replace a document:** `put(project=<key>, slug="<slug>", title="<title>",
  type="<type>", ticket="<id>", body="<full text>")`. A `put` without `mode` replaces the whole body:
  use it only for a new document or a restructure, and `get` the document right before so you don't
  overwrite someone else's change. Always pass `slug` and `title`.
- **Append to the journal:** `put(project=<key>, slug="sdd-journal", title="sdd journal",
  type="journal", mode="append", body="<new entry>")`.
- **Reading a whole change:** `get(type="context", project=<key>, ticket="<id>")` returns every
  document of the change at once, the holdouts included. Plan and build never use it; they get each
  document by its slug.

## Documents

| Slug | Type | Ticket | What |
|---|---|---|---|
| `project` | `project` | | stack, commands, conventions, rules |
| `spec-<capability>` | `spec` | | living spec: how the system behaves now |
| `<id>-proposal` | `plan` | `<id>` | why, scope, acceptance criteria, state |
| `<id>-delta` | `spec-delta` | `<id>` | changes to the specs, one section per capability |
| `<id>-design` | `design` | `<id>` | tier L only |
| `<id>-verification` | `scenario` | `<id>` | scenarios, coverage, results |
| `<id>-holdouts` | `holdout` | `<id>` | scenarios only the reviewer reads |
| `<id>-tasks` | `tasks` | `<id>` | the task list |
| `<id>-review` | `code_review` | `<id>` | review rounds, written by build |
| `<id>-code-review` | `code_review` | `<id>` | the code review, written by `sdd:code_review` |
| `<id>-debrief` | `debrief` | `<id>` | written by debrief |
| `status` | `status` | | current state |
| `runbook` | `runbook` | | how to do recurring operational work |
| `sdd-journal` | `journal` | | append only: what happened |
| `memory` | `memory` | | how we work together |

- `<id>`: kebab-case, named for the work (`add-recurring-tasks`).
- `<capability>`: kebab-case, named for a lasting behavior of the system (`task-completion`), not
  for the work. Reuse an existing capability's exact name rather than inventing a near duplicate.
