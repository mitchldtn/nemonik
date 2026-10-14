---
name: sdd-build
description: How sdd implements an approved change test-first, logs decisions, runs the full verification checklist, and after review applies the delta to the living specs and archives the change.
---

# sdd build

Formats are in `sdd-artifacts`. The rule: nothing is done until the full verification below has run
in this session.

## 1. Pre-flight

- Read `sdd/changes/<id>/`: proposal, delta specs, design, verification, tasks. Not `holdouts.md`.
- The proposal's State must be `approved`. If it isn't, stop and say so.
- Read `sdd/project.md` and `sdd/memory/memory.md`.
- `git status` and `git branch`: note the branch. If there are unrelated uncommitted changes, ask
  before mixing work into them.
- If `tasks.md` has no `**Base:**` line holding a commit (`git cat-file -e <sha>^{commit}`), set it
  to `**Base:** <git rev-parse HEAD>` under the title. The reviewer diffs against it.
- Run the project's test suite once so you know its starting state.
- Look for an existing library or pattern before writing commodity code (parsing, dates, files).

## 2. Build, task by task

For each unchecked task in `tasks.md`:

1. Write the failing test first and run it to see it fail.
2. Write the code that makes it pass.
3. Run the whole test suite after every file you change, not just at the end. Fix any other test
   you broke before moving on.
4. Refactor only on green.
5. Tick the task (`- [x]`).

Skip test-first only if the human says so.

## 3. Log decisions as you go

Any choice you make without the human goes in design's `## Decisions` (or a `## Decisions` section at
the end of `tasks.md` when there is no design), in the format from `sdd-artifacts`:

- `implementation-detail`: log it.
- `design-choice`: log it; the human sees it at debrief.
- `contract-change` (a public interface, a file format, a schema): log it and tell the human now.
  If unsure, treat it as a contract change.

Things you can't decide alone but that don't block you go in `## Discussion`.

Scope: if the work turns out to need something the proposal didn't plan, ask before doing it, and log
it as a decision. If the delta specs or verification need to change, edit them and say so.

## 4. Verify (every item, with evidence)

1. **Self-review:** read the full diff (`git diff` plus new files, except `sdd/changes/<id>/holdouts.md`)
   against each acceptance criterion.
   Look for hardcoded values, missed callers, leftover debug code, unused imports.
2. **Scenarios:** run every Level A scenario in `verification.md` literally, step by step. Write
   pass or fail and the evidence under `## Results`.
3. **Tests:** the full suite passes.
4. **Boundaries:** if you added a type, a field, a value or a file format, every place that reads or
   validates it is updated.
5. **Coverage:** every row in the coverage table is backed by a passing scenario or test.
6. **Scope:** the diff matches the proposal's Scope; anything extra is logged as a decision.
7. **Structure:** the structure check in `sdd-artifacts` passes.

Fix any failure before going on. Set the proposal's State to `built`.

## 5. Fold into the living specs

Only after the review is APPROVED, or skipped for tier S with the human's agreement:

1. Apply each delta to `sdd/specs/` as described in `sdd-artifacts`.
2. Run the structure check on the living specs you changed.
3. Move `sdd/changes/<id>/` to `sdd/changes/archive/YYYY-MM-DD-<id>/` and set State to `archived`.
4. Update `sdd/memory/status.md`: this change is built, reviewed and folded into the specs. Leave
   `journal.md` and `memory.md` to debrief.
5. Tell the human, in a few lines: what was built, test and scenario results, the review outcome,
   any contract-change decision.
