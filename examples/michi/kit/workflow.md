This is your workflow. Follow it in order — do not jump to code.

0. ORIENT. Read `STATUS.md` cold before asserting any state, as well as `memory.md` and `docs/code-style.md` if they exist. Assess Complexity/Uncertainty (Low/Mixed/High) and set pace (Crawl to Fly); a first epic in a repo is Paired/Walk. Before touching shared code, check for in-flight work (open branches, `docs/epics/*/plans/`, gh issues).

1. PLAN. Invoke /michi-planning. Explore, surface assumptions, and clarify anything with two reasonable answers before deciding. Invoke /michi-scenario-test-builder to co-design Level A/B/C verification scenarios. Write the plan to `docs/epics/<epic>/plans/<ticket>-plan.md` and STOP for my sign-off — the plan is a contract. No code before I approve it.

2. BUILD. Invoke /michi-session. Small steps: TDD, run tests after every file change, log decisions into the plan doc. Keep business logic pure, push effects to the edges. Stay in the approved scope — log any expansion as a decision.

3. VERIFY — the Iron Law: no "done" without it. Re-read the diff vs acceptance criteria, execute every Level-A scenario, unit + integration tests green, cross-package schemas updated, no hardcoded caller assumptions, scope check. For multi-repo / new-type / schema changes, launch an independent code-review subagent given ONLY the diff + plan + ACs — not your reasoning.

4. REVIEW. Get a second, independent review from the `michi:review` kit in its own session:
   - Find your session id: `tmux display-message -p '#{session_name}'`.
   - From the folder that holds `nemonik.json`, launch it in the background: `nk run michi:review --no-attach --yolo -- Review ticket <ticket> (plan docs/epics/<epic>/plans/<ticket>-plan.md). Reply to session <your-session-id>.` Add `--agent codex` if you are Codex.
   - While it works, `nk read <reviewer-session-id>` shows how far it has got.
   - Its verdict arrives in this session by `nk write`, and the full review is in `docs/epics/<epic>/reviews/<ticket>-review.md`. Address it, log what you changed and what you declined in the plan doc, and reply with `nk write <reviewer-session-id> '<summary of the fixes>'` (the whole message in single quotes, no apostrophes or backticks in it). Repeat until it replies APPROVED, then close the reviewer: `tmux kill-session -t <reviewer-session-id>`.

5. READY TO COMMIT. Stop and hand off with a proposed one-line commit message (ticket key in it, no AI attribution). The human commits and pushes.

6. DEBRIEF. Invoke /michi-debrief. Curate journal, patterns, memory, and STATUS; calibrate trust for next time.

When unsure, clarify before asserting — orient to my most recent instructions first, then the local docs.
