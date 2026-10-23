---
name: sdd-fold
description: How sdd's builder folds an approved change into the living specs under sdd/specs: applies the delta, checks the result, archives the change folder, updates status and reports to the human.
---

# sdd fold

Only after the final review verdict is APPROVED (after live verification, when there is any), or the
review was skipped for tier S with the human's agreement:

1. Apply each `sdd/changes/<id>/specs/<capability>/spec.md`:
   - New capability: create `sdd/specs/<capability>/spec.md`:

     ```markdown
     # <capability>

     ## Purpose
     <the delta's Purpose>

     ## Requirements
     <the delta's ADDED requirements>
     ```
   - Existing capability: apply ADDED (append the block to `## Requirements`), MODIFIED (replace the
     block with the same name), REMOVED (delete the block), RENAMED (change the header).
2. Check each living spec you changed: no `## ADDED`, `## MODIFIED`, `## REMOVED` or `## RENAMED`
   headers, no `TBD`, and every requirement has a SHALL or MUST and at least one `####` scenario
   with WHEN and THEN.
3. Move `sdd/changes/<id>/` to `sdd/changes/archive/YYYY-MM-DD-<id>/` and set State to `archived`.
4. Update `sdd/memory/status.md`: move this change's line from In progress to Waiting on the human
   (reviewed and folded, with where it is deployed), and update Deployed if it changed. Nothing
   else. Leave `journal.md` and `memory.md` to debrief.
5. Tell the human, in a few lines: what was built, test and scenario results, the review outcomes,
   any contract-change decision.
