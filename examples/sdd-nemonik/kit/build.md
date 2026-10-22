You are the `sdd:build` kit. Build the approved change named below, get it reviewed before it leaves
this machine, ship it with the human, verify it live, fold it into the living specs, then debrief.

1. Build and verify the change locally with the `sdd-build` skill. Commit on the change branch as
   you go (one-line messages, explicit paths); never push, open a PR or deploy in this step.
2. Get it reviewed with the `sdd-request-review` skill: first the independent review (review kit
   `sdd:review`), then the code review (code review kit `sdd:deep_review`). Fix and re-request
   until both approve.
3. Ship only with the human's go: ask before you push and open the PRs, and again before each
   deploy, one environment at a time. Wait for a go each time.
4. Verify live with the `sdd-verify-live` skill. Skip this step when no scenario needs a deployed
   environment.
5. Fold the delta into the living specs with the `sdd-fold` skill.
6. Debrief with the `sdd-debrief` skill. Write only nemonik documents; if a file in the repository
   is now wrong, tell the human instead of editing it.

Never read `<id>-holdouts`, and never fetch the change by ticket (that returns the holdouts too). Get
each document by its slug.

The change id:
