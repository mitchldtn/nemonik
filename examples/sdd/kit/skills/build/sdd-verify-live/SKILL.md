---
name: sdd-verify-live
description: How sdd's builder runs the scenarios that need a deployed environment, after both reviews approved and the human deployed, records the results in the change folder, and gets the independent reviewer's final verdict.
---

# sdd verify live

Only after both reviews approved the code and the human has deployed, to each environment the
scenarios name.

1. Read `sdd/changes/<id>/verification.md`. List the Level A scenarios whose Environment is
   `deployed`.
2. Run each one literally, step by step, on the environment it names. Read the output; don't infer a
   result. Clean up any test data you create, by exact id, and confirm none is left.
3. Write pass or fail and the evidence for each under `## Results`.
4. Send the evidence to the independent reviewer's session with `nk write` (short, single quotes, no
   apostrophes or backticks), pointing at `## Results`, and wait for its final verdict. Record the
   round in `review.md`.
5. A failure is a bug like any other: fix it, run the checks `sdd-testing` gives for that fix, get
   the fix reviewed by the reviewer that would have caught it, and wait for the human's go before
   deploying again.
6. After the final APPROVED, close the reviewer as `sdd-request-review` says.
