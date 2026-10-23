---
name: sdd-plan
description: How sdd turns a request into a change folder (proposal, delta specs, design, verification, holdouts, tasks), with each file's format and rules, gets the human's approval, and launches the build kit.
---

# sdd plan

## 1. Orient

- Read `sdd/project.md`, `sdd/memory/status.md` and `sdd/memory/memory.md` if they exist. When the
  change needs an operational recipe (deploy, release, test setup), read that section of
  `sdd/memory/runbook.md`.
- `ls sdd/specs` and `ls sdd/changes` to see the capabilities and the changes in progress.
- No `sdd/` folder: say so and ask whether to create it (`sdd/project.md` from this template,
  filled in with the human's answers, plus empty `specs/`, `changes/`, `memory/`). Don't create it
  without a yes.

  ```markdown
  # Project

  ## Product
  What it is and who uses it, in two or three sentences.

  ## Stack
  Languages, frameworks, main libraries, where it runs.

  ## Commands
  - Test: `...`
  - Lint / format: `...`
  - Run: `...`

  ## Conventions
  Code style, naming, layout, error handling, test organization.

  ## Rules
  - Always: ...
  - Never: ...
  ```
- A change for the same request already exists: ask whether to continue it or start a new one.

## 2. Understand the request

- If the request is vague, ask what the human wants and why before anything else.
- Pick the change id (kebab-case, named for the work).

## 3. Explore, read only

- Read the code the change touches: entry points, the modules involved, their tests, nearby
  patterns to copy. For a large area, use a subagent to map it and report back.
- Read every living spec the change could touch, in full.
- Research how the problem is usually solved: industry standards, established patterns, and how
  well-known libraries and tools behave (web search for current information). The project's own
  patterns and rules win. The project's own code is not a source for `## Based on`.

## 4. Size it

- Complexity (codebase size, how tight the constraints are, dependencies) and Uncertainty (how clear
  the requirements are, test coverage, new territory) combine into Low, Mixed or High.
- Mixed or High: tell the human and suggest a slower pace (Crawl or Walk).
- Choose the tier and write only its documents:

  | Tier | When | Documents |
  |---|---|---|
  | S | a fix, a tweak or a small behavior change within one component | proposal, tasks, and a delta when behavior changes |
  | M | a feature, or a behavior change across components | proposal, delta, verification, holdouts, tasks |
  | L | crosses modules, adds a dependency, migrates data, or has a real design question | M + design |

  A change that alters no behavior (refactor, tooling, docs) sets `Spec change: none` and writes no
  delta at any tier.

## 5. Assumptions and questions

- List what you are assuming about the system, the users and every boundary the change touches.
  Check each one you can against the code. Mark the rest "to verify in build".
- Ask about anything with two reasonable answers that changes scope, behavior or the acceptance
  criteria. Decide small details yourself and note them as assumptions.
- One question at a time, at the end of your turn, with enough context to answer it cold.

## 6. Write the change, in order

Write each file under `sdd/changes/<id>/`. Re-read each finished file from disk before writing the
next one; the human may have edited it.

### 6.1 `proposal.md`

```markdown
# <id>

**State:** proposed            (proposed → approved → built → archived)
**Tier:** S | M | L
**Spec change:** yes | none
**Complexity/Uncertainty:** Low | Mixed | High, with one line of reasoning
**Pace:** Crawl | Walk | Run

## Why
One or two sentences: the problem and why now.

## Based on
Tiers M and L: how this problem is usually solved, one line per source (with a link) and
what the change takes from it. The code review checks the code against these.

## What changes
- Bullets. Mark breaking changes **BREAKING**.

## Capabilities
- New: `<capability>`: one line.
- Modified: `<capability>`: one line on how its requirements change.

## Scope
## Out of scope
## Assumptions
- A1: <assumption>. Checked how, or "to verify in build".

## Acceptance criteria
1. Specific and checkable.

## Open questions
Only questions that can wait without changing the specs, the approach or the tasks. Omit if none.
```

The Capabilities list decides the delta: one delta file per capability listed, and none that isn't
listed.

### 6.2 Delta specs: `specs/<capability>/spec.md`

One file per capability, one at a time; say which capability before each. Only the sections you need:

```markdown
# <capability>

## Purpose
(new capability only) One or two sentences.

## ADDED Requirements

### Requirement: <Name>
The system SHALL <one behavior>.

#### Scenario: <Name>
- **WHEN** <condition or action>
- **THEN** <observable outcome>
- **AND** <more outcome, optional>

## MODIFIED Requirements

### Requirement: <exact existing name>
<the whole requirement as it should read after the change, every scenario included>

## REMOVED Requirements

### Requirement: <exact existing name>
**Reason:** why it goes.
**Migration:** what users do instead.

## RENAMED Requirements
- FROM: <old name>
- TO: <new name>
```

- A requirement is a behavior contract: what users or other systems can observe, with no class
  names, libraries or implementation steps. One behavior per requirement, SHALL or MUST, under 500
  characters; edge cases go in scenarios. Each has at least one `####` scenario with WHEN and THEN.
  Its name is its identity.
- MODIFIED means: copy the entire existing block from the living spec, then edit it. A partial block
  loses what you left out. If you only add behavior, use ADDED.
- A capability with no living spec yet: the delta creates it, with ADDED requirements only for the
  behavior the change adds or changes, not the existing behavior it relies on.

### 6.3 `design.md` (tier L only)

```markdown
# <id> design

## Context
## Goals / non-goals
## Decisions
### <Decision title>
- **Decision:** what was chosen
- **Alternatives:** what else was considered
- **Reasoning:** why
- **Impact:** implementation-detail | design-choice | contract-change
- **Reversible:** yes | no

## Risks
- <risk> → <mitigation>

## Discussion
Items for the human that don't block the work.
```

The design follows the `## Based on` sources or says why not.

### 6.4 `verification.md` (tiers M and L)

Write one to three scenarios, each for behavior that crosses parts of the system; how to find and
write them is in `sdd-scenarios`. Co-design them with the human: show them and ask whether they
capture what "working" means.

```markdown
# <id> verification

## Coverage
| Requirement (capability) | Change | Covered by | Level |
|---|---|---|---|
| <Name> (<capability>) | ADDED | S1 | A |
| <Name> (<capability>) | MODIFIED | test: <test name> | A |

## Scenarios

### S1: <short name>
**Story:** two to four sentences: who, what they are trying to do, why it matters.
**What could go wrong:** the integration gap this exposes.
**Level:** A (agent runs it, binary result) | B (agent runs it, judgment needed) | C (human judges)
**Environment:** deployed (<environment>) (only for a scenario that needs one)

    Given <precondition>
    When  <action>
    Then  <observable outcome>

- **pass:** specific and observable
- **fail:** specific and observable
- **trap:** a result that looks like a pass but isn't

## Results
(build fills this in)
```

Every ADDED or MODIFIED requirement has a Coverage row, covered by a scenario or a named test; one
scenario or test may cover several requirements.

### 6.5 `holdouts.md` (tiers M and L)

One or two scenarios in the verification format; how to aim them is in `sdd-scenarios`.
Tell the human they may add their own before approving.

### 6.6 `tasks.md`

```markdown
# <id> tasks

## 1. <Group>
- [ ] 1.1 <task>. Verify: <command or observable result>
- [ ] 1.2 ...
```

- Order groups by dependency. Each task is small enough for one sitting.
- Each task's Verify is a command or an observable result. Name a test in it only for new logic.
  Tests call the code's functions directly. They don't run the program the way a user does (a CLI
  command, an HTTP request, a page in a browser) and check what comes back: scenarios do that
  against the real program. Never name a test for what can't break (a field passed through, a
  constant, frozen data), what only checks a mock, or a case another test covers.
- No task runs the full suite, and no task runs the scenarios: build does both once, in local
  verification.
- Each group carries the docs its own work needs.
- Never write `**Base:**`; build adds it.

## 7. Check your own work

- Every capability in the proposal has a delta file, and every delta file is listed.
- Every requirement has a SHALL or MUST and at least one `####` scenario with WHEN and THEN.
- Every MODIFIED, REMOVED or RENAMED name matches an existing living requirement exactly.
- Every ADDED or MODIFIED requirement is in the coverage table (tiers M and L).
- No placeholders, no `TBD`, no vague acceptance criteria. Each acceptance criterion has a scenario
  or task that proves it; each scenario can be run against the program as the tasks leave it.
- Tiers M and L: `## Based on` names real sources.
- Everything significant you read (specs, code findings, the human's answers) shows up somewhere in
  the change.

## 8. Ask for approval

Summarize in a few lines: the id, the tier, the files, the capabilities touched, any open question.
Ask the human to review `sdd/changes/<id>/` and reply "approved" or with changes. Say that this
session ends once build starts, and that replying "approved, keep this session" keeps it open. Then
stop.

If they ask for changes, make them, run the check again, and ask again.

## 9. On "approved"

1. Set the proposal's State to `approved`.
2. From the folder that holds `nemonik.json`, launch the build kit your workflow names:
   - `nk run <build-kit> --new -- <id>`
   - add `--agent codex` if you are Codex, so the chain stays on one agent.
   - add `--project <p>` if the human gave you one.
   - add `--yolo` only if the human asked for an unattended build (no permission prompts).
   - If `--new` fails because `new_window_command` isn't set, run it without `--new`:
     `nk run <build-kit> -- <id>`. The human's window switches to build; `nk run` still prints
     "Started detached session", because every session starts detached.
   - Only if the human asked to run it in the background, use `--no-attach` instead and give the
     human the `nk attach <session>` command it prints.
3. Once `nk run` has succeeded, end this session:
   `[ -n "$TMUX_PANE" ] && tmux kill-session -t "$TMUX_PANE"`. If the human replied "approved, keep
   this session", you used `--no-attach`, or `$TMUX_PANE` is empty, don't: tell them where build is
   running and end your turn. If `nk run` failed, keep this session and show the human the error; if
   the error says the session is still running, don't launch again, give them its `nk attach`
   command.
