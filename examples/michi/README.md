# The michi paired kit and a background reviewer, keeping plans and memory in local files

[michi](https://github.com/csepulv/michi) as two local kits on Claude Code or Codex, with plans,
scenarios, journal, status and memory stored as Markdown files in your repo.

For michi with the nemonik context store instead of files, run the hosted `mitchldtn/michi:paired` kit
(needs a nemonik account; run `nemonik login` first).

## The kits

| Kit | Does |
|---|---|
| `michi:paired` | One paired session that follows the workflow: orient, plan (stop for your sign-off), build, verify, review, ready to commit, debrief. Uses the `michi-planning`, `michi-scenario-test-builder`, `michi-session` and `michi-debrief` skills. |
| `michi:review` | An independent reviewer in its own session. Started by `michi:paired` in the background, it reviews the uncommitted change against the plan, runs the tests and Level A scenarios, writes the review, and sends the verdict back with `nk write`. |

```
you ──nk run michi:paired──> orient → plan ──(you sign off)──> build → verify
                                                                   │
                     nk run michi:review --no-attach --yolo ───────┤
       michi:review ──nk write <session> verdict──> michi:paired   │  nk read <review> to check on it
                     <──nk write <review> fixes────                │
                                                                   ▼
                                           ready to commit (you commit) → debrief
```

## Set up

```bash
nemonik example michi
cp -R michi/nemonik.json michi/kit /path/to/your/repo/
cd /path/to/your/repo
nk verify michi:paired
nk verify michi:review
```

Pin each kit once. The first time your agent opens the repo, it asks whether to trust the folder.

## Run it

```bash
nk run michi:paired -- DEV-12 add recurring tasks
```

It stops twice for you: to sign off the plan, and at "ready to commit". After you commit, tell the
same session to run the debrief.

## Where things go

| What | File |
|---|---|
| Project status | `STATUS.md` |
| How we work together | `memory.md` |
| Applied coding principles | `docs/code-style.md` |
| Milestone plan | `docs/epics/<epic>/plans/<ticket>-plan.md` |
| Scenario catalog, test plan | `docs/epics/<epic>/verification/scenarios.md`, `test-plan.md` |
| Epic journal | `docs/epics/<epic>/journal.md` |
| Review | `docs/epics/<epic>/reviews/<ticket>-review.md` |
| Debrief | `docs/epics/<epic>/debriefs/<ticket>-debrief.md` |

## The files in this example

| File | What it is |
|---|---|
| `nemonik.json` | The two kits. |
| `kit/prompt.md` | michi's doctrine for paired mode: principles, ground rules, verification strategy, how to work. |
| `kit/workflow.md` | The ordered steps `michi:paired` follows. |
| `kit/skills/paired/` | The four michi skills, with the plan and scenario templates included. |
| `kit/review-prompt.md`, `kit/review-workflow.md` | The reviewer's persona, review template and steps. |

## Limits

- **Review runs with `--yolo`**, so it runs your tests without permission prompts.

## Credits

These kits are adapted from [michi](https://github.com/csepulv/michi), an MIT-licensed project by Chris
Sepulveda. See `NOTICE.md`.
