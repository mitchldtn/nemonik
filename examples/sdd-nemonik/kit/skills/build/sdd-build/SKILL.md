---
name: sdd-build
description: How sdd implements an approved change, commits on the change branch, logs decisions and fills in the local verification block before review, all in the nemonik store.
---

# sdd build

The rule: nothing is done until the local verification below has run in this session, and nothing
ships until both reviews approve.

## 1. Pre-flight

- Read the project key from `nemonik.json`.
- `get` by slug: `<id>-proposal`, `<id>-delta`, `<id>-design` (if any), `<id>-verification`,
  `<id>-tasks`. Never `<id>-holdouts`, and never the whole ticket.
- The proposal's State must be `approved`. If it isn't, stop and say so.
- `get` `project` and `memory`.
- `git status` and `git branch`: note the branch. If there are unrelated uncommitted changes, ask
  before mixing work into them.
- If `<id>-tasks` has no `**Base:**` line holding a commit (`git cat-file -e <sha>^{commit}`), add
  `**Base:** <git rev-parse HEAD>` under the title with a `mode="edit"` put. The reviewer diffs
  against it.
- Look for an existing library or pattern before writing commodity code (parsing, dates, files).

## 2. Build, task by task

Ticks are written per group, so the first group with an unticked task may be partly done. Before
starting it, run each of its unticked tasks' `Verify:`; tick the ones that pass in one `edits` put
and continue from the first one that fails.

For each unchecked task in `<id>-tasks`:

1. Decide with `sdd-testing` whether it needs a test; where it does, write it first and see it fail.
2. Write the code, then run the checks `sdd-testing` gives for a change. Fix any test you broke.
3. Refactor only on green.
4. When the last task of a group (a `## N.` heading in `<id>-tasks`) is done, tick the group's tasks
   (`- [ ]` to `- [x]`) in one `mode="edit"` put with an `edits` list.

## 3. Log decisions as you go

Any choice you make without the human goes in `<id>-design`'s `## Decisions` (or a `## Decisions`
section at the end of `<id>-tasks` when there is no design), in this format:

    ### <Decision title>
    - **Decision:** what was chosen
    - **Alternatives:** what else was considered
    - **Reasoning:** why
    - **Impact:** implementation-detail | design-choice | contract-change
    - **Reversible:** yes | no

- `implementation-detail`: log it.
- `design-choice`: log it; the human sees it at debrief.
- `contract-change` (a public interface, a file format, a schema): log it and tell the human now.
  If unsure, treat it as a contract change.

Things you can't decide alone but that don't block you go in `## Discussion`.

Scope: if the work turns out to need something the proposal didn't plan, ask before doing it, and log
it as a decision. If the delta or verification need to change, edit them and say so.

## 4. Verify locally

Fill in this block under `## Results` in `<id>-verification` (at the end of `<id>-tasks` for tier
S), in one `edits` put or a full `put` right after a `get`, and paste it into your summary to the
human:

    Local verification
    - Self-review: <the full diff against each acceptance criterion: met, or what you fixed>
    - Cut: <tests and code removed, or "nothing">
    - Scenarios: <for each local scenario: the command run against the real program, its output, pass or fail>
    - Suite: <the command and its result; run once>
    - Boundaries: <each reader and validator of a new type, field, value or format updated, or "none added">
    - Coverage: <for each coverage row, the scenario or test that passed>
    - Scope: <matches the proposal, or the decision that covers the extra>

- **Cut:** remove the tests `sdd-testing` rules out and any code beyond the proposal's Scope.
- **Scenarios:** run each Level A scenario not marked `deployed` literally, step by step, against
  the real program, never by running a test.

Every line gets a value; fix any failure before going on. Set the proposal's State to `built`, and
commit on the change branch. Don't push, open a PR or deploy: that waits for both reviews and the
human's go.
