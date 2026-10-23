# Plan, build and review kits (michi + OpenSpec) that keep living specs in local files

sdd is a spec-driven workflow that combines [michi](https://github.com/csepulv/michi)'s discipline
(pacing, assumptions, scenario design, full verification, independent review, debrief and memory)
with [OpenSpec](https://github.com/Fission-AI/OpenSpec)'s structure (living capability specs, delta
specs, one folder per change). Four local kits run it on Claude Code or Codex.

For the same workflow with the nemonik context store instead of files, see `sdd-nemonik`.

## The kits

| Kit | Does | Writes |
|---|---|---|
| `sdd:plan` | Explores the code, asks about anything unclear, writes the change folder, stops for your approval, then launches build | `sdd/changes/<id>/` |
| `sdd:build` | Builds the change (tests for new logic and seams only), verifies it locally, gets both reviews, ships with your go, verifies it live, applies the delta to the living specs, archives the change, then debriefs | code, tests, `sdd/specs/`, `sdd/changes/archive/`, `sdd/memory/` |
| `sdd:review` | Reviews the change cold in its own session: proposal, then spec delta, then code; runs the scenarios and the holdouts against the real program, not the test suite; sends a verdict back, and a final one after the live evidence | nothing (replies with `nk write`) |
| `sdd:code_review` | Reads the code after `sdd:review` approves it, without running anything: correctness, over-engineering, what to delete | `sdd/changes/<id>/code-review.md` |

```
sdd:plan ──(you approve)──> sdd:build <──nk write──> sdd:review, then sdd:code_review
                                │
              both approve ─> you say ship ─> live checks ─> fold, debrief
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
nk verify sdd:code_review
```

The first time your agent opens the repo, it asks whether to trust the folder.

## Run a change

```bash
nk run sdd:plan -- add recurring tasks
```

1. Plan reads `sdd/project.md`, your specs and memory, explores the code, and asks you questions one
   at a time. It writes `sdd/changes/add-recurring-tasks/` and asks you to review it.
2. Reply `approved`. Plan launches build in a new window, which needs `new_window_command`
   ([window commands](https://docs.nemonik.io/config/#window-commands)); without it, build takes
   over your window. Plan then closes itself.
3. Build works through the tasks, committing on the change branch, and verifies the change
   locally. It launches the independent review, then the code review, each in the background, and
   goes back and forth with each until it replies APPROVED. Build shows you each review finding with
   its recommendation and fixes only what you approve. Nothing is pushed or deployed yet.
4. Build asks you before it pushes, opens PRs or deploys. After a deploy it runs the scenarios that
   need the deployed environment and sends the evidence to the independent reviewer for its final
   verdict.
5. Build updates `sdd/specs/`, archives the change and debriefs (what was learned, what the change
   made untrue, `sdd/memory/`), then commits those `sdd/` files on the change branch and pushes them
   with your go.

Add `--agent codex` to the first command to run the whole chain on Codex; each kit passes it on.

## Changing the order

Each kit's workflow (`kit/plan.md`, `kit/build.md`, `kit/review.md`, `kit/code-review.md`) sets the
order of its steps and names the next kit; the skills describe the steps. To skip the code review,
edit `kit/build.md`, for example:

```
1. Build and verify the change locally with the `sdd-build`, `sdd-testing` and `sdd-artifacts` skills.
2. Get it reviewed with the `sdd-request-review` skill: the independent review only (review kit
   `sdd:review`). Fix and re-request until it approves.
3. Ship only with the human's go: ask before you push and open the PRs, and again before each
   deploy, one environment at a time. Wait for a go each time.
4. Verify live with the `sdd-verify-live` skill.
5. Fold the delta into the living specs with the `sdd-fold` skill.
6. Debrief with the `sdd-debrief` skill.
7. Commit the `sdd/` files that fold and debrief changed on the change branch, `holdouts.md`
   included (add it by path without opening it), and push them on the human's go.
```

Each kit has its own system prompt (`kit/plan-prompt.md`, `kit/build-prompt.md`,
`kit/review-prompt.md`, `kit/code-review-prompt.md`): what sdd is, the kit's role and limits, and the
principles it uses. The `## Persona` section of `kit/build-prompt.md` is the author's own preference: a functional
programming style (pure functions, explicit effects, types and laws). Replace it with your team's
coding standards; the other kits have no persona.

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
│   ├── review.md                        review rounds
│   └── code-review.md                   the code review
├── changes/archive/
└── memory/status.md, runbook.md, journal.md, memory.md
```

## Change sizes

Plan sizes each change and writes only the files its tier needs:

| Tier | When | Files |
|---|---|---|
| S | a fix, a tweak or a small behavior change within one component | proposal, tasks, and delta specs when behavior changes |
| M | a feature, or a behavior change across components | S plus delta specs, verification (one to three scenarios) and holdouts (at most two) |
| L | crosses modules, adds a dependency, migrates data, or has a real design question | M plus design |

The formats are in `kit/skills/plan/sdd-plan/SKILL.md`.

## The files in this example

| File | What it is |
|---|---|
| `nemonik.json` | The four kits. |
| `kit/plan-prompt.md`, `build-prompt.md`, `review-prompt.md`, `code-review-prompt.md` | Each kit's system prompt: what sdd is, the kit's role and limits, its principles (build's also holds the persona, the code review's the review template). |
| `kit/plan.md`, `build.md`, `review.md`, `code-review.md` | Each kit's workflow, put in the first message (`workflow_inject: "user"`), followed by your request or the change id. |
| `kit/skills/shared/sdd-artifacts` | Where every file lives and how changes and capabilities are named. |
| `kit/skills/shared/sdd-testing` | Which tests to write and which checks to run when, for build and review. |
| `kit/skills/plan/` | `sdd-plan` (the planning steps) and `sdd-scenarios` (how to write scenarios and holdouts). |
| `kit/skills/build/` | `sdd-build` (build and verify locally), `sdd-request-review` (launch the reviewers and answer them), `sdd-verify-live` (run the deployed scenarios), `sdd-fold` (fold into the specs) and `sdd-debrief`. |
| `kit/skills/review/sdd-review` | The reviewer's steps. |
| `sdd/` | Starting files to copy into your repo. |

## Limits

- **Holdouts are written by the plan agent.** Add your own to `holdouts.md` before approving.
- **Both reviews run with `--yolo`**, so the independent review runs your program without
  permission prompts.

## Credits

Adapted from [michi](https://github.com/csepulv/michi) (Chris Sepulveda) and
[OpenSpec](https://github.com/Fission-AI/OpenSpec) (OpenSpec Contributors), both MIT. See
`NOTICE.md`.
