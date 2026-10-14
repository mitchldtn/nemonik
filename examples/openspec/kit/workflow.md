Before anything else:

1. Run `openspec --version`. If it fails, say the CLI is missing (`brew install openspec` or
   `npm install -g @fission-ai/openspec`) and stop.
2. Run `openspec list --json` and read `root`. If it is `null`, or the `openspec-*` skills are not
   available to you, this repo isn't set up: ask whether to run `openspec init --tools claude`
   (Claude Code) or `openspec init --tools codex` (Codex), wait for the answer, and after running it
   tell the human to restart the session so the new skills load.
3. Otherwise run `openspec list --specs` and `openspec list` and summarize, in a few lines, the
   capabilities and the changes in progress.
4. Ask what the human wants to do: explore an idea (`openspec-explore`), propose a change
   (`openspec-propose`), apply a change in progress (`openspec-apply-change`) or archive it
   (`openspec-archive-change`). Then use that skill.

If the human's request is below, use it to choose the skill instead of asking.
