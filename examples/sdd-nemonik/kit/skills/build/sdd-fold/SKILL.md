---
name: sdd-fold
description: How sdd's builder folds an approved change into the living specs in the nemonik store: applies the delta, runs the structure check, archives the proposal, updates status and reports to the human.
---

# sdd fold

Read the project key from `nemonik.json`; the calls are in `sdd-artifacts`. Only after the final
review verdict is APPROVED (after live verification, when there is any), or the review was skipped
for tier S with the human's agreement:

1. Apply the delta to the `spec-<capability>` documents as described in `sdd-artifacts`.
2. Run the structure check on the specs you changed.
3. Set the proposal's State to `archived`.
4. Update `status` in one `mode="edit"` put with an `edits` list: move this change's line from In
   progress to Waiting on the human (reviewed and folded, with where it is deployed), and update
   Deployed if it changed. Nothing else. Leave `sdd-journal` and `memory` to debrief.
5. Tell the human, in a few lines: what was built, test and scenario results, the review outcomes,
   any contract-change decision.
