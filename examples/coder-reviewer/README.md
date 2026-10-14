# Two local kits, a coder and a reviewer, that review each other's work

The coder makes a change and launches the reviewer; the reviewer sends its
verdict back; they go back and forth until the reviewer replies APPROVED. The
whole loop is written in the two prompts: each agent messages the other's tmux
session with `nk write`. You start it with one command.

## Prerequisites

- tmux.
- Claude Code (or Codex with `--agent codex`).

## Run it

```bash
nemonik example coder-reviewer
cd coder-reviewer

# Pin both kits once (press y at each prompt).
nk verify coder
nk verify reviewer

# Start the coder with a task. It launches the reviewer itself.
nk run coder --yolo "Add input validation to src/login.ts."
```

To work on your own code, copy `nemonik.json` and `kit/` into that repo and run
from there. `nemonik.json` is found by walking up from the current directory.

`--yolo` lets each agent run `nk run` and `nk write` without asking you
first, and the coder starts the reviewer with `--yolo` too. It also skips every
other permission prompt in both sessions, so only use it on code and a machine
where that is acceptable. Without it, approve those commands as they come up.

## The files

| File | What it does |
| --- | --- |
| `nemonik.json` | Declares two prompt-only local kits, `coder` and `reviewer`. |
| `kit/coder.md` | Makes the change, finds its own session id, launches the reviewer with `--no-attach` and tells it where to reply. |
| `kit/reviewer.md` | Reviews the change and sends the verdict, with its own session id, to the coder's session. |

## Watch it

The coder runs in your terminal. The reviewer runs in the background; list its
session with `tmux ls` and watch it with `nk attach <session>` (detach
again with `Ctrl-b d`).
