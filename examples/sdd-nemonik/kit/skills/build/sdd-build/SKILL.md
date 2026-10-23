---
name: sdd-build
description: How sdd implements an approved change test-first, commits on the change branch, logs decisions and runs the full local verification checklist before review, all in the nemonik store.
---

# sdd build

Documents, formats and the read/write calls are in `sdd-artifacts`. The rule: nothing is done until
the local verification below has run in this session, and nothing ships until both reviews approve.

## 1. Pre-flight

- Read the project key from `nemonik.json`.
- `get` by slug: `<id>-proposal`, `<id>-delta`, `<id>-design` (if any), `<id>-verification`,
  `<id>-tasks`. Never `<id>-holdouts`, and never the whole ticket.
- The proposal's State must be `approved`. If it isn't, stop and say so.
- `get` `project` and `memory`.
- `git status` and `git branch`: note the branch. If there are unrelated uncommitted changes, ask
  before mixing work into them.
- If `<id>-tasks` has no `**Base:**` line holding a commit (`git cat-file -e <sha>^{commit}`), add
  `**Base:** <git rev-parse HEAD>` under the title with a `mode="edit"` put. The reviewer diffs
  against it.
- Run the project's test suite once so you know its starting state.
- Look for an existing library or pattern before writing commodity code (parsing, dates, files).

## 2. Build, task by task

Ticks are written per group, so the first group with an unticked task may be partly done. Before
starting it, run each of its unticked tasks' `Verify:`; tick the ones that pass in one `edits` put
and continue from the first one that fails.

For each unchecked task in `<id>-tasks`:

1. Decide whether it needs a test. Test new logic and the seams between components, and extend an
   existing test before adding a new one. Don't test what can't break (a field passed through
   unchanged, a constant) or what only checks a mock, and don't write a test that repeats a
   scenario's steps: the scenario run is the check.
2. Where it needs a test, write the failing test first and run it to see it fail.
3. Write the code that makes it pass.
4. After each change, run the tests relevant to the code you changed, not the whole suite. Fix any
   test you broke before moving on.
5. Refactor only on green.
6. When the last task of a group (a `## N.` heading in `<id>-tasks`) is done, tick the group's tasks
   (`- [ ]` to `- [x]`) in one `mode="edit"` put with an `edits` list.

Skip test-first only if the human says so.

## 3. Log decisions as you go

Any choice you make without the human goes in `<id>-design`'s `## Decisions` (or a `## Decisions`
section at the end of `<id>-tasks` when there is no design), in the format from `sdd-artifacts`:

- `implementation-detail`: log it.
- `design-choice`: log it; the human sees it at debrief.
- `contract-change` (a public interface, a file format, a schema): log it and tell the human now.
  If unsure, treat it as a contract change.

Things you can't decide alone but that don't block you go in `## Discussion`.

Scope: if the work turns out to need something the proposal didn't plan, ask before doing it, and log
it as a decision. If the delta or verification need to change, edit them and say so.

## 4. Verify locally (every item, with evidence)

1. **Self-review:** read the full diff (`git diff` plus new files) against each acceptance criterion.
   Look for hardcoded values, missed callers, leftover debug code, unused imports.
2. **Cut:** review the diff for tests and code the change doesn't need (tests that repeat another
   test or a scenario's steps, test what can't break or only check a mock; code beyond the proposal's
   Scope), remove them, and tell the human what you removed, or that you removed nothing.
3. **Scenarios:** run every Level A scenario whose Environment is `local` (or missing), literally,
   step by step, against the real program, never by running a test. Write pass or fail under
   `## Results` with the command you ran and the output you read, all scenarios of the round in one
   put (an `edits` put, or a full `put` right after a `get`, as in `sdd-artifacts`).
4. **Tests:** run the full suite, once; it passes.
5. **Boundaries:** if you added a type, a field, a value or a file format, every place that reads or
   validates it is updated.
6. **Coverage:** every row in the coverage table is backed by a passing scenario or test.
7. **Scope:** the diff matches the proposal's Scope; anything extra is logged as a decision.
8. **Structure:** the structure check in `sdd-artifacts` passes.

Fix any failure before going on. Set the proposal's State to `built`, and commit on the change
branch. Don't push, open a PR or deploy: that waits for both reviews and the human's go.
