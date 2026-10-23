# Plan, build and review kits (michi + OpenSpec) that keep living specs in the nemonik context store

sdd combines [michi](https://github.com/csepulv/michi)'s discipline (pacing, assumptions, scenario
design, full verification, independent review, debrief and memory) with
[OpenSpec](https://github.com/Fission-AI/OpenSpec)'s structure (living capability specs, delta
specs, one record per change). Four local kits run it on Claude Code or Codex, with every document
in the nemonik context store instead of files in your repo.

## The kits

| Kit | Does | Writes (nemonik documents) |
|---|---|---|
| `sdd:plan` | Explores the code, asks about anything unclear, writes the change, stops for your approval, then launches build | `<id>-proposal`, `-delta`, `-design`, `-verification`, `-holdouts`, `-tasks` |
| `sdd:build` | Builds the change (tests for new logic and seams only), verifies it locally, gets both reviews, ships with your go, verifies it live, applies the delta to the living specs, then debriefs | code and tests (files), `spec-<capability>`, `<id>-review`, `<id>-debrief`, `sdd-journal`, `status`, `runbook`, `memory` |
| `sdd:review` | Reviews the change cold in its own session: proposal, then delta, then code; runs the scenarios and the holdouts against the real program, not the test suite; sends a verdict back, and a final one after the live evidence | nothing (replies with `nk write`) |
| `sdd:code_review` | Reads the code after `sdd:review` approves it, without running anything: correctness, over-engineering, what to delete | `<id>-code-review` |

```
sdd:plan ──(you approve)──> sdd:build <──nk write──> sdd:review, then sdd:code_review
                                │
              both approve ─> you say ship ─> live checks ─> fold, debrief
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
nk verify sdd:code_review
```

The first time your agent opens the repo, it asks whether to trust the folder.

## Run a change

```bash
nk run sdd:plan -- add recurring tasks
```

1. Plan reads the project's `project`, `status` and `memory` documents and its specs, explores the
   code, and asks you questions one at a time. It writes the change documents and asks you to review
   them.
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
5. Build updates the `spec-<capability>` documents, marks the change archived and debriefs: what
   was learned, what the change made untrue, and the project's `status`, `runbook`, `sdd-journal`
   and `memory`.

Add `--agent codex` to the first command to run the whole chain on Codex; each kit passes it on.

## Changing the order

Each kit's workflow (`kit/plan.md`, `kit/build.md`, `kit/review.md`, `kit/code-review.md`) sets the
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
```

Each kit has its own system prompt (`kit/plan-prompt.md`, `kit/build-prompt.md`,
`kit/review-prompt.md`, `kit/code-review-prompt.md`): what sdd is, the kit's role and limits, and the
principles it uses. The `## Persona` section of `kit/build-prompt.md` is a basic coding persona;
replace it with your own standards.

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
| `<id>-review`, `<id>-code-review`, `<id>-debrief` | review rounds; the code review; the debrief |
| `status` | current state, under 100 lines, edited in place |
| `runbook` | how to do recurring operational work |
| `sdd-journal`, `memory` | what happened, how we work |

Every change document carries the ticket `<id>`.

## Change sizes

Plan sizes each change and writes only the documents its tier needs:

| Tier | When | Documents |
|---|---|---|
| S | a fix, a tweak or a small behavior change within one component | `<id>-proposal`, `<id>-tasks` |
| M | a feature, or a behavior change across components | S plus `<id>-delta`, `<id>-verification` (one to three scenarios) and `<id>-holdouts` (at most two) |
| L | crosses modules, adds a dependency, migrates data, or has a real design question | M plus `<id>-design` |

The formats are in `kit/skills/shared/sdd-artifacts/SKILL.md`.

## The files in this example

| File | What it is |
|---|---|
| `nemonik.json` | The four kits and your nemonik project key. |
| `kit/plan-prompt.md`, `build-prompt.md`, `review-prompt.md`, `code-review-prompt.md` | Each kit's system prompt: what sdd is, the kit's role and limits, its principles (build's also holds the persona, the code review's the review template). |
| `kit/plan.md`, `build.md`, `review.md`, `code-review.md` | Each kit's workflow, put in the first message, followed by your request or the change id. |
| `kit/mcps.json` | The nemonik MCP server every kit attaches. |
| `kit/skills/` | The shared `sdd-artifacts` skill (documents, formats, calls) and each phase's skills. Build's are `sdd-build`, `sdd-request-review`, `sdd-verify-live`, `sdd-fold` and `sdd-debrief`. |

## Limits

- **Holdouts are written by the plan agent.** Add your own to `<id>-holdouts` before approving.
- **The store keeps no history of a document.** A `put` replaces the body.
- **Both reviews run with `--yolo`**, so the independent review runs your program without
  permission prompts.

## Credits

Adapted from [michi](https://github.com/csepulv/michi) (Chris Sepulveda) and
[OpenSpec](https://github.com/Fission-AI/OpenSpec) (OpenSpec Contributors), both MIT. See
`NOTICE.md`.
