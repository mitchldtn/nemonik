---
name: sdd-artifacts
description: The formats of every sdd file (living specs, change proposals, delta specs, design, verification, holdouts, tasks, memory), the size tiers, and the structure check. Use whenever you read or write anything under sdd/.
---

# sdd artifacts

Read only the section for the file you are about to write.

## Layout

```
sdd/
├── project.md
├── specs/<capability>/spec.md          living specs
├── changes/<id>/                       one change in progress
│   ├── proposal.md
│   ├── specs/<capability>/spec.md      delta specs
│   ├── design.md
│   ├── verification.md
│   ├── holdouts.md
│   ├── tasks.md
│   └── review.md                       written by build from review's verdict
├── changes/archive/YYYY-MM-DD-<id>/    finished changes, same files
└── memory/
    ├── status.md
    ├── runbook.md
    ├── journal.md
    └── memory.md
```

- `<id>`: kebab-case, named for the work (`add-recurring-tasks`).
- `<capability>`: kebab-case, named for a lasting behavior of the system (`task-completion`), not
  for the work. Reuse an existing capability's exact folder name rather than inventing a near
  duplicate. List them with `ls sdd/specs`.

## Size tiers

Choose the tier in the proposal and write only its files.

| Tier | When | Files |
|---|---|---|
| S | small fix or tweak, behavior barely changes | proposal, tasks |
| M | a feature or a behavior change | proposal, delta specs, verification, holdouts, tasks |
| L | crosses modules, adds a dependency, migrates data, or has a real design question | M + design |

A change that alters no behavior (refactor, tooling, docs) sets `Spec change: none` in the proposal
and writes no delta specs at any tier.

## Project: `sdd/project.md`

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

## Living spec: `sdd/specs/<capability>/spec.md`

A behavior contract: what users or other systems can observe. No class names, libraries or
implementation steps; those belong in design.

```markdown
# <capability>

## Purpose
One or two sentences on what this capability is for.

## Requirements

### Requirement: <Name>
The system SHALL <one behavior>.

#### Scenario: <Name>
- **WHEN** <condition or action>
- **THEN** <observable outcome>
- **AND** <more outcome, optional>
```

Rules:
- One behavior per requirement, SHALL or MUST, description under 500 characters. Edge cases go in
  scenarios, not the description.
- Every requirement has at least one scenario. Scenario headers use exactly four `#`.
- A requirement's name is its identity: deltas and verification refer to it by name.

## Proposal: `sdd/changes/<id>/proposal.md`

```markdown
# <id>

**State:** proposed            (proposed → approved → built → archived)
**Tier:** S | M | L
**Spec change:** yes | none
**Complexity/Uncertainty:** Low | Mixed | High, with one line of reasoning
**Pace:** Crawl | Walk | Run

## Why
One or two sentences: the problem and why now.

## What changes
- Bullets. Mark breaking changes **BREAKING**.

## Capabilities
- New: `<capability>`: one line.
- Modified: `<capability>`: one line on how its requirements change.

## Scope
What this change delivers.

## Out of scope
What it explicitly does not.

## Assumptions
- A1: <assumption>. Checked how, or "to verify in build".

## Acceptance criteria
1. Specific and checkable.

## Open questions
Only questions that can wait without changing the specs, the approach or the tasks. Omit if none.
```

The Capabilities list is the contract between the proposal and the delta specs: one delta file per
capability listed, and none that isn't listed.

## Delta spec: `sdd/changes/<id>/specs/<capability>/spec.md`

What the change does to one capability. Use only the sections you need.

```markdown
# <capability> delta

## Purpose
(new capability only) One or two sentences.

## ADDED Requirements

### Requirement: <Name>
The system SHALL ...

#### Scenario: <Name>
- **WHEN** ...
- **THEN** ...

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

MODIFIED means: copy the entire existing block from the living spec, then edit it. A partial block
loses the parts you left out when the delta is applied. If you are only adding behavior, use ADDED.

## Design: `sdd/changes/<id>/design.md` (tier L)

```markdown
# <id> design

## Context
Current state and constraints. Point to the proposal for the why instead of repeating it.

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

Build appends to Decisions and Discussion while it works. For tiers S and M without a design file,
build keeps a `## Decisions` and `## Discussion` section at the end of `tasks.md` instead.

## Verification: `sdd/changes/<id>/verification.md` (tiers M and L)

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

    Given <precondition>
    When  <action>
    Then  <observable outcome>

- **pass:** specific and observable
- **fail:** specific and observable
- **trap:** a result that looks like a pass but isn't

## Results
(build fills this in: each scenario pass or fail, with the evidence)
```

Every ADDED or MODIFIED requirement appears in Coverage with at least one scenario or a named test.

## Holdouts: `sdd/changes/<id>/holdouts.md` (tiers M and L)

Two or three scenarios in the verification format, aimed at the shortcuts an implementer takes:
tests that always pass, expectations derived from the code instead of the spec, doing the easy part
and skipping the hard part, output that reads well but is wrong. Only the review kit reads this file.

## Tasks: `sdd/changes/<id>/tasks.md`

```markdown
# <id> tasks

## 1. <Group>
- [ ] 1.1 <task>. Verify: <test, command or observable result>
- [ ] 1.2 ...

## 2. <Group>
- [ ] 2.1 ...
```

- Order groups by dependency. Each task is small enough for one sitting.
- Each task says how to verify it. Write the failing test first.
- Each group carries the tests and docs its own work needs. No "write all tests" group at the end.
- Only `- [x]` counts as done.
- Build adds `**Base:** <sha>` under the title at pre-flight; plan never writes it.

## Review: `sdd/changes/<id>/review.md`

Written by build from the reviewer's messages: one section per round with the findings, what build
changed, and the final verdict.

## Memory: `sdd/memory/`

Change these files line by line; don't rewrite a whole file to change one line.

- `status.md`: what is true now, read at the start of every session. Under 100 lines.

  ```markdown
  # Status

  **Last updated:** YYYY-MM-DD

  ## In progress
  ## Waiting on the human
  ## Next
  ## Deployed
  ## Recently shipped
  ## Open follow-ups
  ```

  Exactly these sections, one line per item, linking the file that holds the detail. No history
  (that is the journal and the archived changes), no recipes (`runbook.md`), no rules
  (`project.md`, `memory.md`). Drop a Recently shipped line two weeks after its release.
- `runbook.md`: how to do recurring operational work (deploys, releases, test setups, access) and the
  environment facts it needs. One `##` section per area; read only the section you need.
- `journal.md`: append only. `### YYYY-MM-DD: <id>` per entry: what happened, what was learned.
- `memory.md`: how we work together: corrections from the human, confirmed approaches, gotchas. One
  `##` section per topic; merge a new entry into its topic, never add a dated section. Remove
  entries that stop being true.

## Applying a delta (build, after review approves)

For each `sdd/changes/<id>/specs/<capability>/spec.md`:

1. New capability: create `sdd/specs/<capability>/spec.md` with the delta's Purpose and its ADDED
   requirements under `## Requirements`.
2. ADDED: append each requirement block to the living spec's `## Requirements`.
3. MODIFIED: replace the living block with the same name by the delta's block.
4. REMOVED: delete the living block with that name.
5. RENAMED: change the requirement's header from the old name to the new one.

Then move `sdd/changes/<id>/` to `sdd/changes/archive/YYYY-MM-DD-<id>/` and set the proposal's State
to `archived`.

## Structure check

Run before handing anything off:

- [ ] Every capability in the proposal has a delta file, and every delta file is listed.
- [ ] Every requirement has a SHALL or MUST and at least one `####` scenario with WHEN and THEN.
- [ ] Every MODIFIED, REMOVED or RENAMED name matches an existing living requirement exactly.
- [ ] Every ADDED or MODIFIED requirement is in the coverage table.
- [ ] After applying: the living spec has no `## ADDED`, `## MODIFIED`, `## REMOVED` or `## RENAMED`
      headers, and no `TBD`.
