---
name: sdd-fold
description: How sdd's builder folds an approved change into the living specs in the nemonik store: applies the delta, checks the result, archives the proposal, updates status and reports to the human.
---

# sdd fold

Read the project key from `nemonik.json`; the calls are in `sdd-artifacts`. Only after the final
review verdict is APPROVED (after live verification, when there is any), or the review was skipped
for tier S with the human's agreement:

1. Apply each capability section of `<id>-delta`:
   - New capability: `put` `spec-<capability>` (type `spec`, title `Spec: <capability>`):

     ```markdown
     # <capability>

     ## Purpose
     <the delta's Purpose>

     ## Requirements
     <the delta's ADDED requirements>
     ```
   - Existing capability: `get` `spec-<capability>`, then apply ADDED (append the block), MODIFIED
     (replace the block with the same name), REMOVED (delete the block), RENAMED (change the header),
     and `put` the whole spec back.
2. Check each spec you changed: no `## ADDED`, `## MODIFIED`, `## REMOVED` or `## RENAMED` headers,
   no `TBD`, and every requirement has a SHALL or MUST and at least one `####` scenario with WHEN
   and THEN.
3. Set the proposal's State to `archived` with a `mode="edit"` put. Archived changes stay in the
   store under their ticket; nothing moves.
4. Update `status` in one `mode="edit"` put with an `edits` list: move this change's line from In
   progress to Waiting on the human (reviewed and folded, with where it is deployed), and update
   Deployed if it changed. Nothing else. Leave `sdd-journal` and `memory` to debrief.
5. Tell the human, in a few lines: what was built, test and scenario results, the review outcomes,
   any contract-change decision.
