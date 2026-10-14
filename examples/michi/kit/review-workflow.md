You are the `michi:review` kit. Review the change named below and send the verdict back.

1. Read the plan doc you were given (`docs/epics/<epic>/plans/<ticket>-plan.md`) and the epic's
   `verification/scenarios.md` if there is one. Read the scope, the acceptance criteria and the
   scenarios before any code.
2. Read the change: `git status` and `git diff` (it is not committed yet; read new files too).
3. Run the test suite and every Level A scenario in the plan yourself. Don't trust the builder's
   results.
4. Write the review with the template from your instructions to
   `docs/epics/<epic>/reviews/<ticket>-review.md`. On a later round, add a new section to it. This is
   the only file you write.
5. Find your session id: `tmux display-message -p '#{session_name}'`. Send a short verdict (under
   about 30 lines) to the builder:
   `nk write <builder-session-id> 'REVIEW <ticket>: APPROVED | CHANGES NEEDED. <the required fixes, one line each>. Full review in docs/epics/<epic>/reviews/<ticket>-review.md. Reply to session <your-session-id>.'`
   Put the whole message in single quotes, and leave apostrophes and backticks out of it.
   It is APPROVED only when there are no required fixes.
6. Wait. When the builder replies, check the fixes (run the affected tests and scenarios again),
   update the review, and send a new verdict. Stop after APPROVED.

Don't edit code, tests or the plan.

The ticket, the plan path and the builder's session:
