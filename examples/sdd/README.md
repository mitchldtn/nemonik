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
| `sdd:build` | Builds the change test-first and verifies it locally, gets both reviews, ships with your go, verifies it live, applies the delta to the living specs, archives the change, then debriefs | code, tests, `sdd/specs/`, `sdd/changes/archive/`, `sdd/memory/` |
| `sdd:review` | Reviews the change cold in its own session: proposal, then spec delta, then code; runs the scenarios and the holdouts; sends a verdict back, and a final one after the live evidence | nothing (replies with `nk write`) |
| `sdd:deep_review` | Reviews the code after `sdd:review` approves it: correctness, simplicity, tests, risks | `sdd/changes/<id>/deep-review.md` |

```
sdd:plan ──(you approve)──> sdd:build <──nk write──> sdd:review, then sdd:deep_review
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
nk verify sdd:deep_review
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
3. Build works through the tasks test-first, committing on the change branch, and verifies the
   change locally. It launches the independent review, then the code review, each in the
   background, and goes back and forth with each until it replies APPROVED. Build shows you each
   review finding with its recommendation and fixes only what you approve. Nothing is pushed or
   deployed yet.
4. Build asks you before it pushes, opens PRs or deploys. After a deploy it runs the scenarios that
   need the deployed environment and sends the evidence to the independent reviewer for its final
   verdict.
5. Build updates `sdd/specs/`, archives the change and debriefs (what was learned, what the change
   made untrue, `sdd/memory/`), then commits those `sdd/` files on the change branch and pushes them
   with your go.

Add `--agent codex` to the first command to run the whole chain on Codex; each kit passes it on.

## Changing the order

Each kit's workflow (`kit/plan.md`, `kit/build.md`, `kit/review.md`, `kit/deep-review.md`) sets the
order of its steps and names the next kit; the skills describe the steps. To skip the code review,
edit `kit/build.md`, for example:

```
1. Build and verify the change locally with the `sdd-build` skill.
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
│   ├── review.md                        review rounds
│   └── deep-review.md                   the code review
├── changes/archive/
└── memory/status.md, runbook.md, journal.md, memory.md
```

Small changes write less: tier S is just a proposal and tasks. The formats are in
`kit/skills/shared/sdd-artifacts/SKILL.md`.

## The files in this example

| File | What it is |
|---|---|
| `nemonik.json` | The four kits. |
| `kit/prompt.md` | The system prompt every kit shares: a persona and the principles. |
| `kit/plan.md`, `build.md`, `review.md`, `deep-review.md` | Each kit's workflow (`deep-review.md` also holds the review template), put in the first message (`workflow_inject: "user"`), followed by your request or the change id. |
| `kit/skills/shared/sdd-artifacts` | Every file format, the size tiers, how a delta is applied, the structure check. |
| `kit/skills/plan/` | `sdd-plan` (the planning steps) and `sdd-scenarios` (how to write scenarios and holdouts). |
| `kit/skills/build/` | `sdd-build` (build and verify locally), `sdd-request-review` (launch the reviewers and answer them), `sdd-verify-live` (run the deployed scenarios), `sdd-fold` (fold into the specs) and `sdd-debrief`. |
| `kit/skills/review/sdd-review` | The reviewer's steps. |
| `sdd/` | Starting files to copy into your repo. |

## Limits

- **Holdouts are written by the plan agent.** Add your own to `holdouts.md` before approving.
- **Both reviews run with `--yolo`**, so they run your tests without permission prompts.

## Credits

Adapted from [michi](https://github.com/csepulv/michi) (Chris Sepulveda) and
[OpenSpec](https://github.com/Fission-AI/OpenSpec) (OpenSpec Contributors), both MIT. See
`NOTICE.md`.
