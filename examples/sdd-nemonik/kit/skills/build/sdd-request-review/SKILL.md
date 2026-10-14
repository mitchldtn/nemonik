---
name: sdd-request-review
description: How sdd's builder gets an independent review. It launches the review kit in the background, waits for the verdict sent with nk write, records each round in the nemonik store, replies with its fixes, and closes the reviewer after APPROVED.
---

# sdd request review

Read the project key from `nemonik.json`; the calls are in `sdd-artifacts`. Tier S may skip the
review if the human agrees. Tiers M and L always do it.

1. Find your own session id: `tmux display-message -p '#{session_name}'`.
2. From the folder that holds `nemonik.json`, launch the review kit your workflow names, in the
   background:

       nk run <review-kit> --no-attach --yolo -- Review change <id>. Reply to session <your-session-id>.

   Add `--agent codex` if you are Codex.
3. Wait. The verdict arrives as a message in this session, with the reviewer's session id. To see
   how far the reviewer has got, `nk read <reviewer-session-id>` prints its screen.
4. Record the round in `<id>-review`: the findings, what you changed, what you declined and why.
5. If the verdict is not APPROVED: fix what you agree with (back through the verification in
   `sdd-build`), then reply with `nk write <reviewer-session-id> '<short summary of the changes>'`
   (the whole message in single quotes, no apostrophes or backticks in it). Repeat.
6. If a finding needs the human (a disagreement, a scope question), ask them instead of guessing.
7. After APPROVED, close the reviewer: `tmux kill-session -t <reviewer-session-id>`.
