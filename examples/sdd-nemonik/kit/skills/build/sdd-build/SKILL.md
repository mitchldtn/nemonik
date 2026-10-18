---
name: sdd-build
description: How sdd implements an approved change test-first, logs decisions, runs the full verification checklist, and after review applies the delta to the living specs and archives the change, all in the nemonik store.
---

# sdd build

Documents, formats and the read/write calls are in `sdd-artifacts`. The rule: nothing is done until
the full verification below has run in this session.

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

1. Write the failing test first and run it to see it fail.
2. Write the code that makes it pass.
3. Run the whole test suite after every file you change, not just at the end. Fix any other test
   you broke before moving on.
4. Refactor only on green.
5. When the last task of a group (a `## N.` heading in `<id>-tasks`) is done, tick the group's tasks
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

## 4. Verify (every item, with evidence)

1. **Self-review:** read the full diff (`git diff` plus new files) against each acceptance criterion.
   Look for hardcoded values, missed callers, leftover debug code, unused imports.
2. **Scenarios:** run every Level A scenario in `<id>-verification` literally, step by step. Write
   pass or fail and the evidence under `## Results`, all scenarios of the round in one put (an
   `edits` put, or a full `put` right after a `get`, as in `sdd-artifacts`).
3. **Tests:** the full suite passes.
4. **Boundaries:** if you added a type, a field, a value or a file format, every place that reads or
   validates it is updated.
5. **Coverage:** every row in the coverage table is backed by a passing scenario or test.
6. **Scope:** the diff matches the proposal's Scope; anything extra is logged as a decision.
7. **Structure:** the structure check in `sdd-artifacts` passes.

Fix any failure before going on. Set the proposal's State to `built`.

## 5. Fold into the living specs

Only after the review is APPROVED, or skipped for tier S with the human's agreement:

1. Apply the delta to the `spec-<capability>` documents as described in `sdd-artifacts`.
2. Run the structure check on the specs you changed.
3. Set the proposal's State to `archived`.
4. Update `status` in one `mode="edit"` put with an `edits` list: move this change's line from In
   progress to Waiting on the human (reviewed and folded, waiting for the commit), and update
   Deployed if it changed. Nothing else. Leave `sdd-journal` and `memory` to debrief.
5. Tell the human, in a few lines: what was built, test and scenario results, the review outcome,
   any contract-change decision.
