---
name: sdd-plan
description: How sdd turns a request into a change stored as nemonik documents (proposal, delta, design, verification, holdouts, tasks), gets the human's approval, and launches the build kit.
---

# sdd plan

Documents, formats and the read/write calls are in `sdd-artifacts`; scenario writing is in
`sdd-scenarios`.

## 1. Orient

- Read the project key from `nemonik.json`. Missing or still `your-nemonik-project`: ask for it.
- `get` the `project`, `status` and `memory` documents. Search the `spec` documents to see the
  capabilities. When the change needs an operational recipe (deploy, release, test setup), read
  that section of `runbook`.
- No `project` document: say so, ask the human the questions in its template (product, stack,
  commands, conventions, rules), and `put` it once they answer.
- A change for the same request already exists (`get` `<id>-proposal`): ask whether to continue it
  or start a new one.

## 2. Understand the request

- If the request is vague, ask what the human wants and why before anything else.
- Pick the change id (kebab-case, named for the work).

## 3. Explore, read only

- Read the code the change touches: entry points, the modules involved, their tests, nearby
  patterns to copy. For a large area, use a subagent to map it and report back.
- `get` every living spec the change could touch, in full.
- Research how the problem is usually solved: industry standards, established patterns, and how
  well-known libraries and tools behave (web search for current information). The project's own
  patterns and rules win. Tiers M and L record the sources in the proposal's `## Based on`, one line
  each: the source, with a link, and what the change takes from it. The project's own code is not
  a source here.
- A capability the change touches has no spec yet: the delta creates it. Its ADDED requirements cover
  the existing behavior the change relies on as well as the new behavior; say which are which in the
  proposal.

## 4. Size it

- Complexity (codebase size, how tight the constraints are, dependencies) and Uncertainty (how clear
  the requirements are, test coverage, new territory) combine into Low, Mixed or High.
- Mixed or High: tell the human and suggest a slower pace (Crawl or Walk).
- Choose the tier (S, M, L) and whether behavior changes at all.

## 5. Assumptions and questions

- List what you are assuming about the system, the users and every boundary the change touches.
  Check each one you can against the code. Mark the rest "to verify in build".
- Ask about anything with two reasonable answers that changes scope, behavior or the acceptance
  criteria. Decide small details yourself and note them as assumptions.
- One question at a time, at the end of your turn, with enough context to answer it cold.

## 6. Write the change, in order

`put` each document with its slug, title, type and `ticket="<id>"` (table in `sdd-artifacts`).
Re-`get` each finished document before writing the next one; the human may have edited it.

1. `<id>-proposal`. Its Capabilities list decides the delta's sections.
2. `<id>-delta`, one capability section at a time. Say which capability before each.
3. `<id>-design` (tier L only).
4. `<id>-verification`, with the `sdd-scenarios` skill. Co-design the scenarios with the human: show
   them and ask whether they capture what "working" means.
5. `<id>-holdouts` (tiers M and L), with the `sdd-scenarios` skill.
6. `<id>-tasks`.

## 7. Check your own work

- Run the structure check in `sdd-artifacts`.
- No placeholders, no `TBD`, no vague acceptance criteria.
- Each acceptance criterion has a scenario or task that proves it; each scenario has a task that
  builds what it checks.
- Tiers M and L: `## Based on` names real sources, and the design follows them or says why not.
- Everything significant you read (specs, code findings, the human's answers) shows up somewhere in
  the change.
- `git status` shows nothing new: plan writes no files.

## 8. Ask for approval

Summarize in a few lines: the id, the tier, the documents, the capabilities touched, any open
question. Ask the human to review the documents (in the nemonik app, or `get` them) and reply
"approved" or with changes. Say that this session ends once build starts, and that replying
"approved, keep this session" keeps it open. Then stop.

If they ask for changes, make them, run the check again, and ask again.

## 9. On "approved"

1. Set the proposal's State to `approved` with a `mode="edit"` put.
2. From the folder that holds `nemonik.json`, launch the build kit your workflow names:
   - `nk run <build-kit> --new -- <id>`
   - add `--agent codex` if you are Codex, so the chain stays on one agent.
   - add `--yolo` only if the human asked for an unattended build (no permission prompts).
   - If `--new` fails because `new_window_command` isn't set, run it without `--new`:
     `nk run <build-kit> -- <id>`. The human's window switches to build; `nk run` still prints
     "Started detached session", because every session starts detached.
   - Only if the human asked to run it in the background, use `--no-attach` instead and give the
     human the `nk attach <session>` command it prints.
3. Once `nk run` has succeeded, end this session:
   `[ -n "$TMUX_PANE" ] && tmux kill-session -t "$TMUX_PANE"`. If the human replied "approved, keep
   this session", you used `--no-attach`, or `$TMUX_PANE` is empty, don't: tell them where build is
   running and end your turn. If `nk run` failed, keep this session and show the human the error; if
   the error says the session is still running, don't launch again, give them its `nk attach`
   command.
