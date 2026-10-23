---
name: sdd-request-review
description: How sdd's builder gets its change reviewed before it ships: the independent review first, then the code review, each in its own session, verdicts sent with nk write, each round recorded, findings reported to the human for a decision before any fix, and when each reviewer is closed.
---

# sdd request review

Tier S may skip both reviews if the human agrees. Tiers M and L run every review the workflow
names. Nothing is pushed, opened as a PR or deployed until both approve.

1. Find your own session id: `tmux display-message -p '#{session_name}'`.
2. From the folder that holds `nemonik.json`, launch the independent review, the review kit your
   workflow names, in the background:

       nk run <review-kit> --no-attach --yolo -- Review change <id>. Reply to session <your-session-id>. Deployed later: <S-numbers, or none>.

   Add `--agent codex` if you are Codex, and `--project <p>` if you were started with one. List the
   scenarios whose Environment is `deployed`, so the reviewer approves the code without them and
   waits for their evidence.
3. Wait. The verdict arrives as a message in this session, with the reviewer's session id. To see
   how far the reviewer has got, `nk read <reviewer-session-id>` prints its screen.
4. Record the round in `sdd/changes/<id>/review.md`: the findings, what you changed, what you
   declined and why.
5. If the verdict has findings, change nothing yet. Report them to the human, each with:
   - **Real:** the input or state that gives a wrong result, and whether you reproduced it. None
     means polish.
   - **Already covered:** an existing rule, step or line that handles it, or "no".
   - **Smallest fix:** the smallest change that removes the failure, its size, and any new concept,
     step or tool it adds.
   - **Practice:** what established tools or docs do (one cited source), or the project rule that
     applies.
   - **Recommendation:** fix (the smallest version), skip, or ask, with one line of why.

   Wait for the human's decision on each. Fix only what they approved, then re-run the tests and
   scenarios that cover the changed code, and the full suite once if code changed (nothing for a
   docs-only fix). Record each decision in `review.md`, then reply with
   `nk write <reviewer-session-id> '<what was fixed, and what the human declined>'` (the whole
   message in single quotes, no apostrophes or backticks in it). Repeat. A verdict with no findings
   needs no report. A finding the human declined is settled: if a reviewer withholds approval only
   over declined findings, the human decides whether to ship.
6. When the independent review approves the code and the human has decided its findings, launch
   the code review, the code review kit your workflow names, the same way:

       nk run <code-review-kit> --no-attach --yolo -- Code review change <id>. Reply to session <your-session-id>.

   Its full review lands in `sdd/changes/<id>/code-review.md`; the message carries the summary.
   Handle its rounds as in steps 4 and 5, recording what you changed and declined in `review.md`.
   If its fixes change the code, send them to the independent reviewer too and wait for its verdict.
7. Close each reviewer when it is done, unless the human has a window attached to it. This one
   command does both:

       [ -z "$(tmux list-clients -t <reviewer-session-id>)" ] && tmux kill-session -t <reviewer-session-id>

   The code reviewer closes after APPROVED. The independent reviewer closes last: after its final
   APPROVED and after the code reviewer's APPROVED, so it can check any code review fix.
