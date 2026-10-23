---
name: sdd-review
description: How the sdd reviewer reviews a built change cold, in order (proposal, delta specs, code), runs the scenarios and the holdouts, and sends a verdict back to the builder with nk write.
---

# sdd review

## 1. Read, in this order

1. `sdd/changes/<id>/proposal.md`: is this the right problem, and what is in and out of scope?
2. The delta specs under `sdd/changes/<id>/specs/`: is "done" defined correctly? Compare with the
   living specs in `sdd/specs/` they change.
3. `design.md` if present, `verification.md`, `tasks.md`.
4. `holdouts.md`: the builder has not seen these.
5. The code: `git diff <base>`, where `<base>` is the `**Base:**` commit in `tasks.md`, shows the
   change, committed or not. Read the new files `git status` lists too. If the base is
   missing, not a commit, or not an ancestor of `HEAD` (`git merge-base --is-ancestor <base> HEAD`),
   read `git diff` and `git status` instead and say so in the verdict.

## 2. Check

- Every ADDED or MODIFIED requirement is implemented and checked by a test or a scenario. Name any
  that isn't.
- Nothing in the code goes beyond the proposal's Scope without a logged decision.
- Tests check behavior from the spec, not just whatever the code happens to do. Look for tests that
  can't fail.
- Tests and code the change doesn't need: tests that repeat another test, test what can't break or
  only check a mock; code beyond the proposal's Scope. Name each one to cut.
- Bugs, edge cases, error handling, security problems, inconsistencies with the project's conventions
  (`sdd/project.md`).
- Read the test suite's result in build's Results; don't run the suite.
- Run every Level A scenario in `verification.md` whose Environment is `local` (or missing), and
  every holdout you can run without a deployed environment, against the real program. Record pass or
  fail with the evidence; don't trust the builder's Results. Scenarios and holdouts that need a
  deployed environment are judged from the builder's evidence after the deploy; re-check them
  read-only where you can.

## 3. Verdict

Sort findings into **blocking** (wrong behavior, a failed scenario or holdout, a requirement not met),
**should fix**, and **nit**. Give each a file and line and the evidence. Each blocking finding names
a concrete failure: the input or state, and the wrong result.

The verdict is `APPROVED` only when nothing is blocking. With deployed scenarios still pending, it
is `APPROVED for code`; the final `APPROVED` comes after their evidence.

## 4. Send it

Find your own session id: `tmux display-message -p '#{session_name}'`. Keep the message short (under
about 30 lines): long messages may arrive as pasted text the builder treats as data. Put the whole
message in single quotes, and leave apostrophes and backticks out of it.

    nk write <builder-session-id> 'REVIEW <id>: APPROVED | APPROVED for code | CHANGES NEEDED. <findings>. Reply to session <your-session-id>.'

Then wait. When the builder replies with changes, check them (run the affected scenarios and holdouts
again) and send a new verdict. Stop after the final APPROVED.
