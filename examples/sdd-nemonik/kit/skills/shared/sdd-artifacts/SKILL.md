---
name: sdd-artifacts
description: The nemonik context documents sdd uses (living specs, change proposals, delta specs, design, verification, holdouts, tasks, memory), how to read and write them, their formats, the size tiers, and the structure check. Use whenever you read or write an sdd document.
---

# sdd artifacts (nemonik store)

Read only the section for the document you are about to write.

## Reading and writing

Every call takes `project=<key>`, the `project` field of `nemonik.json`.

- **Read one document:** `get(type="context", project=<key>, slug="<slug>")`. A "no context found"
  error means the document doesn't exist yet; it isn't a failure.
- **List the specs:** `search(project=<key>, type="spec", since="2000-01-01T00:00:00Z", limit=100)`.
- **Write or replace a document:** `put(project=<key>, slug="<slug>", title="<title>",
  type="<type>", ticket="<id>", body="<full text>")`. A `put` replaces the whole body: to change one
  part, `get` the document, edit it, and `put` all of it back. Always pass `slug` and `title`.
- **Append to the journal:** `put(project=<key>, slug="sdd-journal", title="sdd journal",
  type="journal", mode="append", body="<new entry>")`.
- **Reading a whole change:** `get(type="context", project=<key>, ticket="<id>")` returns every
  document of the change at once, the holdouts included. Plan and build never use it; they get each
  document by its slug.

## Documents

| Slug | Type | Ticket | What |
|---|---|---|---|
| `project` | `project` | | stack, commands, conventions, rules |
| `spec-<capability>` | `spec` | | living spec: how the system behaves now |
| `<id>-proposal` | `plan` | `<id>` | why, scope, acceptance criteria, state |
| `<id>-delta` | `spec-delta` | `<id>` | changes to the specs, one section per capability |
| `<id>-design` | `design` | `<id>` | tier L only |
| `<id>-verification` | `scenario` | `<id>` | scenarios, coverage, results |
| `<id>-holdouts` | `holdout` | `<id>` | scenarios only the reviewer reads |
| `<id>-tasks` | `tasks` | `<id>` | the task list |
| `<id>-review` | `code_review` | `<id>` | review rounds, written by build |
| `<id>-debrief` | `debrief` | `<id>` | written by debrief |
| `status` | `status` | | current state |
| `sdd-journal` | `journal` | | append only: what happened |
| `memory` | `memory` | | how we work together |

- `<id>`: kebab-case, named for the work (`add-recurring-tasks`).
- `<capability>`: kebab-case, named for a lasting behavior of the system (`task-completion`), not
  for the work. Reuse an existing capability's exact name rather than inventing a near duplicate.

## Size tiers

Choose the tier in the proposal and write only its documents.

| Tier | When | Documents |
|---|---|---|
| S | small fix or tweak, behavior barely changes | proposal, tasks |
| M | a feature or a behavior change | proposal, delta, verification, holdouts, tasks |
| L | crosses modules, adds a dependency, migrates data, or has a real design question | M + design |

A change that alters no behavior (refactor, tooling, docs) sets `Spec change: none` in the proposal
and writes no delta at any tier.

## Project: `project`

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

## Living spec: `spec-<capability>`

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

## Proposal: `<id>-proposal`

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
## Out of scope
## Assumptions
- A1: <assumption>. Checked how, or "to verify in build".

## Acceptance criteria
1. Specific and checkable.

## Open questions
Only questions that can wait without changing the specs, the approach or the tasks. Omit if none.
```

The Capabilities list is the contract between the proposal and the delta: one delta section per
capability listed, and none that isn't listed.

## Delta: `<id>-delta`

One `#` section per capability, then only the subsections you need.

```markdown
# <id> delta

# <capability>

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

## Design: `<id>-design` (tier L)

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

Build appends to Decisions and Discussion while it works. Without a design document, build keeps a
`## Decisions` and `## Discussion` section at the end of `<id>-tasks` instead.

## Verification: `<id>-verification` (tiers M and L)

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

## Holdouts: `<id>-holdouts` (tiers M and L)

Two or three scenarios in the verification format, aimed at the shortcuts an implementer takes:
tests that always pass, expectations derived from the code instead of the spec, doing the easy part
and skipping the hard part, output that reads well but is wrong. Only the review kit reads it.

## Tasks: `<id>-tasks`

```markdown
# <id> tasks

## 1. <Group>
- [ ] 1.1 <task>. Verify: <test, command or observable result>
- [ ] 1.2 ...
```

- Order groups by dependency. Each task is small enough for one sitting.
- Each task says how to verify it. Write the failing test first.
- Each group carries the tests and docs its own work needs. No "write all tests" group at the end.
- Only `- [x]` counts as done.
- Build adds `**Base:** <sha>` under the title at pre-flight; plan never writes it.

## Review: `<id>-review`

Written by build from the reviewer's messages: one section per round with the findings, what build
changed, and the final verdict.

## Memory

- `status`: `**Last updated:** YYYY-MM-DD` at the top, then what is in progress, what shipped
  recently, what is next. Rewritten to stay true; old lines removed, not piled up.
- `sdd-journal`: append only. `### YYYY-MM-DD: <id>` per entry: what happened, what was learned.
- `memory`: how we work together: corrections from the human, confirmed approaches, gotchas.
  Remove entries that stop being true.

## Applying the delta (build, after review approves)

For each capability section of `<id>-delta`:

1. New capability: `put` `spec-<capability>` (type `spec`, title `Spec: <capability>`) with the
   delta's Purpose and its ADDED requirements under `## Requirements`.
2. Existing capability: `get` `spec-<capability>`, then apply ADDED (append the block), MODIFIED
   (replace the block with the same name), REMOVED (delete the block), RENAMED (change the header),
   and `put` the whole spec back.

Then set the proposal's State to `archived` (get, edit, put). Archived changes stay in the store under
their ticket; nothing moves.

## Structure check

Run before handing anything off:

- [ ] Every capability in the proposal has a delta section, and every delta section is listed.
- [ ] Every requirement has a SHALL or MUST and at least one `####` scenario with WHEN and THEN.
- [ ] Every MODIFIED, REMOVED or RENAMED name matches an existing living requirement exactly.
- [ ] Every ADDED or MODIFIED requirement is in the coverage table.
- [ ] After applying: the living spec has no `## ADDED`, `## MODIFIED`, `## REMOVED` or `## RENAMED`
      headers, and no `TBD`.
