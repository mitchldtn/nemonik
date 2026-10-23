---
name: sdd-plan
description: How sdd turns a request into a change folder (proposal, delta specs, design, verification, holdouts, tasks), gets the human's approval, and launches the build kit.
---

# sdd plan

Formats are in `sdd-artifacts`; scenario writing is in `sdd-scenarios`.

## 1. Orient

- Read `sdd/project.md`, `sdd/memory/status.md` and `sdd/memory/memory.md` if they exist. When the
  change needs an operational recipe (deploy, release, test setup), read that section of
  `sdd/memory/runbook.md`.
- `ls sdd/specs` and `ls sdd/changes` to see the capabilities and the changes in progress.
- No `sdd/` folder: say so and ask whether to create it (`sdd/project.md` from the Project template
  in `sdd-artifacts`, filled in with the human's answers, plus empty `specs/`, `changes/`,
  `memory/`). Don't create it without a yes.
- A change for the same request already exists: ask whether to continue it or start a new one.

## 2. Understand the request

- If the request is vague, ask what the human wants and why before anything else.
- Pick the change id (kebab-case, named for the work).

## 3. Explore, read only

- Read the code the change touches: entry points, the modules involved, their tests, nearby
  patterns to copy. For a large area, use a subagent to map it and report back.
- Read every living spec the change could touch, in full.
- Research how the problem is usually solved: industry standards, established patterns, and how
  well-known libraries and tools behave (web search for current information). The project's own
  patterns and rules win. Tiers M and L record the sources in the proposal's `## Based on`, one line
  each: the source, with a link, and what the change takes from it. The project's own code is not
  a source here.
- A capability the change touches has no spec yet: the delta creates it, with ADDED requirements only
  for the behavior the change adds or changes, not the existing behavior it relies on.

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

Re-read each finished file from disk before writing the next one; the human may have edited it.

1. `proposal.md`. Its Capabilities list decides which delta files exist.
2. One delta spec per listed capability, one file at a time. Say which capability before each.
3. `design.md` (tier L only).
4. `verification.md`, with the `sdd-scenarios` skill. Co-design the scenarios with the human: show
   them and ask whether they capture what "working" means.
5. `holdouts.md` (tiers M and L), with the `sdd-scenarios` skill.
6. `tasks.md`.

## 7. Check your own work

- Run the structure check in `sdd-artifacts`.
- No placeholders, no `TBD`, no vague acceptance criteria.
- Each acceptance criterion has a scenario or task that proves it; each scenario can be run against
  the program as the tasks leave it, and no task adds a test that repeats a scenario's steps (the
  scenario run is the check). A task's Verify names a test only for new logic or a seam between
  components, never for what can't break or only checks a mock.
- Tiers M and L: `## Based on` names real sources, and the design follows them or says why not.
- Everything significant you read (specs, code findings, the human's answers) shows up somewhere in
  the change.

## 8. Ask for approval

Summarize in a few lines: the id, the tier, the files, the capabilities touched, any open question.
Ask the human to review `sdd/changes/<id>/` and reply "approved" or with changes. Say that this
session ends once build starts, and that replying "approved, keep this session" keeps it open. Then
stop.

If they ask for changes, make them, run the check again, and ask again.

## 9. On "approved"

1. Set the proposal's State to `approved`.
2. From the folder that holds `nemonik.json`, launch the build kit your workflow names:
   - `nk run <build-kit> --new -- <id>`
   - add `--agent codex` if you are Codex, so the chain stays on one agent.
   - add `--project <p>` if the human gave you one.
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
