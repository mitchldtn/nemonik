---
name: sdd-debrief
description: How sdd debriefs a change once it is built, reviewed and folded, in three passes (capture, invalidate, currency), checks the living specs against the code, and updates journal, status and memory.
---

# sdd debrief

Find the change in `sdd/changes/archive/*-<id>/`. Read all of it except `holdouts.md`, including
`review.md` and the decisions, and `git log` for any commits of it.

Scale it: a tier S change gets a short journal entry and passes 2 and 3; tiers M and L get all of it.

## Pass 1: Capture

- **Delivery:** each acceptance criterion: met, partly met, or not. What was cut or deferred, and why.
  Scenario and test results. Review rounds and what they caught.
- **Decisions:** for each logged decision: still right? Should any become a standing rule in
  `sdd/project.md`? Resolve or defer each Discussion item.
- **Bugs:** for each bug found during build or review: how was it found, why did it happen, and which
  scenario would have caught it earlier. Add that scenario to the relevant living spec as a
  `#### Scenario:` if it describes lasting behavior.
- **Learnings:** product or codebase gotchas go in the journal; how-we-work lessons go in memory.

## Pass 2: Invalidate

What did this change make untrue?

- **Living specs vs code:** for every capability the change touched, read the living spec next to the
  code. Each requirement still holds? Fix the spec if it drifted, or report a code bug.
- **Other docs:** README, docs, comments that describe the old behavior. List them for the human.
- **Memory:** entries in `sdd/memory/memory.md` about files, commands or habits that no longer
  exist. Remove or correct them in place.
- **Runbook:** recipes in `sdd/memory/runbook.md` this change made wrong. Correct them in place.

## Pass 3: Currency

- **Status:** read `sdd/memory/status.md` as if new to the project, and change it line by line. Move
  this change to one Recently shipped line pointing at its archived change (or keep it in Waiting on
  the human with what is left, such as a prod deploy). Correct every line that is no longer true,
  drop Recently shipped lines older than two weeks, update `**Last updated:**`. Keep it under 100
  lines and in its six sections; a recipe found there moves to `runbook.md`.
- **Runbook:** a recurring operational recipe learned in this change goes in its `runbook.md` section.
- **Memory:** a new how-we-work lesson is merged into its topic section of `memory.md`.
- **Journal:** add `### YYYY-MM-DD: <id>` with what happened and what was learned.
- **Archive:** mention old archived changes or stale notes the human might want to delete. Don't
  delete them.

## Trust

Say whether the next change can move faster, the same, or slower, and why:

- **Faster:** criteria met without help, decisions sound and logged, scenarios caught something,
  review found little.
- **Slower:** "done" claimed too early, checks skipped, scope crept, specs drifted from the code.

## Outputs

- `sdd/changes/archive/<date>-<id>/debrief.md`: the three passes and the trust call.
- `sdd/memory/journal.md`, `status.md`, `runbook.md`, `memory.md` updated.
- Living spec fixes from pass 2.
- End with the two or three findings that change what happens next.
