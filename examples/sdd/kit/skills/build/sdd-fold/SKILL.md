---
name: sdd-fold
description: How sdd's builder folds an approved change into the living specs under sdd/specs: applies the delta, runs the structure check, archives the change folder, updates status and reports to the human.
---

# sdd fold

Formats are in `sdd-artifacts`. Only after the final review verdict is APPROVED (after live
verification, when there is any), or the review was skipped for tier S with the human's agreement:

1. Apply each delta to `sdd/specs/` as described in `sdd-artifacts`.
2. Run the structure check on the living specs you changed.
3. Move `sdd/changes/<id>/` to `sdd/changes/archive/YYYY-MM-DD-<id>/` and set State to `archived`.
4. Update `sdd/memory/status.md`: move this change's line from In progress to Waiting on the human
   (reviewed and folded, with where it is deployed), and update Deployed if it changed. Nothing
   else. Leave `journal.md` and `memory.md` to debrief.
5. Tell the human, in a few lines: what was built, test and scenario results, the review outcomes,
   any contract-change decision.
