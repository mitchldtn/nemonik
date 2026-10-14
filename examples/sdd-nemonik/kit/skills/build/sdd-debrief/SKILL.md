---
name: sdd-debrief
description: How sdd debriefs a change once it is built, reviewed and folded, in three passes (capture, invalidate, currency), checks the living specs against the code, and updates the journal, status and memory documents in the nemonik store.
---

# sdd debrief

Read the project key from `nemonik.json`; the calls are in `sdd-artifacts`. Get each document by its
slug: `<id>-proposal`, `<id>-delta`, `<id>-design` (if any), `<id>-verification`, `<id>-tasks`,
`<id>-review`. Never `<id>-holdouts`. Also read `git log` for any commits of the change.

Scale it: a tier S change gets a short journal entry and passes 2 and 3; tiers M and L get all of it.

## Pass 1: Capture

- **Delivery:** each acceptance criterion: met, partly met, or not. What was cut or deferred, and why.
  Scenario and test results. Review rounds and what they caught.
- **Decisions:** for each logged decision: still right? Should any become a standing rule in the
  `project` document? Resolve or defer each Discussion item.
- **Bugs:** for each bug found during build or review: how was it found, why did it happen, and which
  scenario would have caught it earlier. Add that scenario to the relevant living spec as a
  `#### Scenario:` if it describes lasting behavior.
- **Learnings:** product or codebase gotchas go in the journal; how-we-work lessons go in memory.

## Pass 2: Invalidate

What did this change make untrue?

- **Living specs vs code:** for every capability the change touched, read `spec-<capability>` next to
  the code. Each requirement still holds? Fix the spec if it drifted, or report a code bug.
- **Other docs:** README, docs, comments in the repository that describe the old behavior. List them
  for the human.
- **Memory:** entries in `memory` about files, commands or habits that no longer exist. Remove or
  correct them.

## Pass 3: Currency

- **Status:** read `status` as if new to the project. Rewrite every line that is no longer true,
  record where this change is now (built, committed, PR open or merged), update `**Last updated:**`.
- **Journal:** append `### YYYY-MM-DD: <id>` to `sdd-journal` with what happened and what was learned.
- **Old documents:** mention archived changes or stale documents the human might want to retire.
  Don't delete anything.

## Trust

Say whether the next change can move faster, the same, or slower, and why:

- **Faster:** criteria met without help, decisions sound and logged, scenarios caught something,
  review found little.
- **Slower:** "done" claimed too early, checks skipped, scope crept, specs drifted from the code.

## Outputs

- `<id>-debrief` (type `debrief`, ticket `<id>`): the three passes and the trust call.
- `sdd-journal`, `status`, `memory` updated.
- Living spec fixes from pass 2.
- End with the two or three findings that change what happens next.
