# Plan, build and review kits (michi + OpenSpec) that keep living specs in local files

sdd is a spec-driven workflow that combines [michi](https://github.com/csepulv/michi)'s discipline
(pacing, assumptions, scenario design, full verification, independent review, debrief and memory)
with [OpenSpec](https://github.com/Fission-AI/OpenSpec)'s structure (living capability specs, delta
specs, one folder per change). Three local kits run it on Claude Code or Codex.

For the same workflow with the nemonik context store instead of files, see `sdd-nemonik`.

## The kits

| Kit | Does | Writes |
|---|---|---|
| `sdd:plan` | Explores the code, asks about anything unclear, writes the change folder, stops for your approval, then launches build | `sdd/changes/<id>/` |
| `sdd:build` | Builds the change test-first, runs the full verification, launches review, applies the delta to the living specs, archives the change, waits for you to commit, then debriefs | code, tests, `sdd/specs/`, `sdd/changes/archive/`, `sdd/memory/` |
| `sdd:review` | Reviews the change cold in its own session: proposal, then spec delta, then code; runs the scenarios and the holdouts; sends a verdict back | nothing (replies with `nk write`) |

```
sdd:plan ──(you approve)──> sdd:build <──nk write──> sdd:review
                                │
                        you commit, build debriefs
```

## Set up

```bash
nemonik example sdd
cp -R sdd/nemonik.json sdd/kit sdd/sdd /path/to/your/repo/
cd /path/to/your/repo
```

Fill in `sdd/project.md` (stack, test command, conventions). Then review and pin each kit once:

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

1. Plan reads `sdd/project.md`, your specs and memory, explores the code, and asks you questions one
   at a time. It writes `sdd/changes/add-recurring-tasks/` and asks you to review it.
2. Reply `approved`. Plan launches build in a new window.
3. Build works through the tasks, launches review in the background, and goes back and forth with it
   until it replies APPROVED. Then it updates `sdd/specs/`, archives the change, and proposes a
   commit message.
4. Commit, then tell build it is committed. Build debriefs (what was learned, what the change made
   untrue, `sdd/memory/`) and proposes a second commit message for those files.

Add `--agent codex` to the first command to run the whole chain on Codex; each kit passes it on.

## Changing the order

Each kit's workflow (`kit/plan.md`, `kit/build.md`, `kit/review.md`) sets the order of its steps and
names the next kit; the skills describe the steps. To commit and open a PR before the review, edit
`kit/build.md`, for example:

```
1. Build and verify the change with the `sdd-build` skill.
2. Commit, push and open a pull request.
3. Get an independent review with the `sdd-request-review` skill. The review kit is `sdd:review`.
4. Fold the delta into the living specs, as `sdd-build` describes, and commit it.
5. Debrief with the `sdd-debrief` skill.
```

The `## Persona` section of `kit/prompt.md` is a basic coding persona; replace it with your own
standards.

## What a change looks like

```
sdd/
├── project.md
├── specs/task-completion/spec.md        how the system behaves now
├── changes/add-recurring-tasks/
│   ├── proposal.md                      why, scope, acceptance criteria, assumptions, capabilities
│   ├── specs/task-completion/spec.md    ADDED / MODIFIED / REMOVED requirements
│   ├── design.md                        how, plus decisions (large changes only)
│   ├── verification.md                  scenarios, levels A/B/C, coverage table
│   ├── holdouts.md                      scenarios only the reviewer runs
│   ├── tasks.md                         checkboxes, each with how to verify it
│   └── review.md                        review rounds
├── changes/archive/
└── memory/status.md, runbook.md, journal.md, memory.md
```

Small changes write less: tier S is just a proposal and tasks. The formats are in
`kit/skills/shared/sdd-artifacts/SKILL.md`.

## The files in this example

| File | What it is |
|---|---|
| `nemonik.json` | The three kits. |
| `kit/prompt.md` | The system prompt every kit shares: a persona and the principles. |
| `kit/plan.md`, `build.md`, `review.md` | Each kit's workflow, put in the first message (`workflow_inject: "user"`), followed by your request or the change id. |
| `kit/skills/shared/sdd-artifacts` | Every file format, the size tiers, how a delta is applied, the structure check. |
| `kit/skills/plan/` | `sdd-plan` (the planning steps) and `sdd-scenarios` (how to write scenarios and holdouts). |
| `kit/skills/build/` | `sdd-build` (build, verify, fold into the specs), `sdd-request-review` (launch the reviewer and answer it) and `sdd-debrief`. |
| `kit/skills/review/sdd-review` | The reviewer's steps. |
| `sdd/` | Starting files to copy into your repo. |

## Limits

- **Holdouts are written by the plan agent.** Add your own to `holdouts.md` before approving.
- **Review runs with `--yolo`**, so it runs your tests without permission prompts.

## Credits

Adapted from [michi](https://github.com/csepulv/michi) (Chris Sepulveda) and
[OpenSpec](https://github.com/Fission-AI/OpenSpec) (OpenSpec Contributors), both MIT. See
`NOTICE.md`.
