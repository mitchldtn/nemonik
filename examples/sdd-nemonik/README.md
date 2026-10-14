# Plan, build and review kits (michi + OpenSpec) that keep living specs in the nemonik context store

sdd combines [michi](https://github.com/csepulv/michi)'s discipline (pacing, assumptions, scenario
design, full verification, independent review, debrief and memory) with
[OpenSpec](https://github.com/Fission-AI/OpenSpec)'s structure (living capability specs, delta
specs, one record per change). Three local kits run it on Claude Code or Codex, with every document
in the nemonik context store instead of files in your repo.

## The kits

| Kit | Does | Writes (nemonik documents) |
|---|---|---|
| `sdd:plan` | Explores the code, asks about anything unclear, writes the change, stops for your approval, then launches build | `<id>-proposal`, `-delta`, `-design`, `-verification`, `-holdouts`, `-tasks` |
| `sdd:build` | Builds the change test-first, runs the full verification, launches review, applies the delta to the living specs, waits for you to commit, then debriefs | code and tests (files), `spec-<capability>`, `<id>-review`, `<id>-debrief`, `sdd-journal`, `status`, `memory` |
| `sdd:review` | Reviews the change cold in its own session: proposal, then delta, then code; runs the scenarios and the holdouts; sends a verdict back | nothing (replies with `nk write`) |

```
sdd:plan ──(you approve)──> sdd:build <──nk write──> sdd:review
                                │
                        you commit, build debriefs
```

## Set up

You need a nemonik account and a project to write into.

```bash
nemonik example sdd-nemonik
cp -R sdd-nemonik/nemonik.json sdd-nemonik/kit /path/to/your/repo/
cd /path/to/your/repo
```

Set `"project"` in `nemonik.json` to your nemonik project key. Then review and pin each kit once:

```bash
nk verify sdd:plan
nk verify sdd:build
nk verify sdd:review
```

The first time your agent opens the repo, it asks whether to trust the folder.

## Run a change

```bash
nk run sdd:plan -- add recurring tasks
```

1. Plan reads the project's `project`, `status` and `memory` documents and its specs, explores the
   code, and asks you questions one at a time. It writes the change documents and asks you to review
   them.
2. Reply `approved`. Plan launches build in a new window.
3. Build works through the tasks, launches review in the background, and goes back and forth with it
   until it replies APPROVED. Then it updates the `spec-<capability>` documents, marks the change
   archived, and proposes a commit message.
4. Commit, then tell build it is committed. Build debriefs: what was learned, what the change made
   untrue, and the project's `status`, `sdd-journal` and `memory`.

Add `--agent codex` to the first command to run the whole chain on Codex; each kit passes it on.

## Changing the order

Each kit's workflow (`kit/plan.md`, `kit/build.md`, `kit/review.md`) sets the order of its steps and
names the next kit; the skills describe the steps. To commit and open a PR before the review, edit
`kit/build.md`, for example:

```
1. Build and verify the change with the `sdd-build` skill.
2. Commit, push and open a pull request.
3. Get an independent review with the `sdd-request-review` skill. The review kit is `sdd:review`.
4. Fold the delta into the living specs, as `sdd-build` describes.
5. Debrief with the `sdd-debrief` skill.
```

The `## Persona` section of `kit/prompt.md` is a basic coding persona; replace it with your own
standards.

## The documents

| Slug | What |
|---|---|
| `project` | stack, commands, conventions, rules |
| `spec-<capability>` | how the system behaves now (type `spec`) |
| `<id>-proposal` | why, scope, acceptance criteria, assumptions, capabilities, state |
| `<id>-delta` | ADDED / MODIFIED / REMOVED requirements per capability |
| `<id>-design` | how, plus decisions (large changes only) |
| `<id>-verification` | scenarios, levels A/B/C, coverage table, results |
| `<id>-holdouts` | scenarios only the reviewer reads |
| `<id>-tasks` | checkboxes, each with how to verify it |
| `<id>-review`, `<id>-debrief` | review rounds; the debrief |
| `status`, `sdd-journal`, `memory` | current state, what happened, how we work |

Every change document carries the ticket `<id>`. The formats are in `kit/skills/shared/sdd-artifacts/SKILL.md`.

## The files in this example

| File | What it is |
|---|---|
| `nemonik.json` | The three kits and your nemonik project key. |
| `kit/prompt.md` | The system prompt every kit shares: a persona and the principles. |
| `kit/plan.md`, `build.md`, `review.md` | Each kit's workflow, put in the first message, followed by your request or the change id. |
| `kit/mcps.json` | The nemonik MCP server every kit attaches. |
| `kit/skills/` | The shared `sdd-artifacts` skill (documents, formats, calls) and each phase's skills. Build's are `sdd-build`, `sdd-request-review` and `sdd-debrief`. |

## Limits

- **Holdouts are written by the plan agent.** Add your own to `<id>-holdouts` before approving.
- **The store keeps no history of a document.** A `put` replaces the body.
- **Review runs with `--yolo`**, so it runs your tests without permission prompts.

## Credits

Adapted from [michi](https://github.com/csepulv/michi) (Chris Sepulveda) and
[OpenSpec](https://github.com/Fission-AI/OpenSpec) (OpenSpec Contributors), both MIT. See
`NOTICE.md`.
