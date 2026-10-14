# Scenario Test Builder

Generate narrative-driven verification scenarios, based on Cem Kaner's scenario testing methodology (2003),
adapted for AI-assisted / autonomous development. **This produces scenarios, plans, and criteria — not test code.**

**Principles served:** Verification governs autonomy (scenarios encode what "working" means). Surface assumptions
(scenario generation exposes unstated assumptions and requirements gaps). Shared context as foundation.

## When to Use

Discovering what to test (before writing test code); building test plans; defining verification scenarios for
autonomous agent work; exploring how a system could fail (adversarial); surfacing requirements gaps.

## What It Produces

Markdown, not code: a **scenario catalog** (`docs/epics/<epic>/verification/scenarios.md`), a
**test plan** (`docs/epics/<epic>/verification/test-plan.md`), and **acceptance criteria** for the plan doc's
`## Scenarios` section. Use the template under **Scenario template** at the end of this skill for the per-scenario block.

## Kaner's Five Quality Criteria

Every scenario is evaluated against five criteria. A good scenario is:

1. **A story** — a person doing something for a reason, with context on why it matters and the consequence of
   failure. Not a mechanical enumeration of inputs/outputs.
2. **Motivating** — a stakeholder with influence would push to fix a program that failed this test.
3. **Credible** — stakeholders believe it will *probably* happen in the real world, not just *could*. Ground in
   observed behavior, specs, competitor usage, or real data patterns.
4. **Complex** — involves multiple features/data flows/components. Single-feature checks belong in unit/domain
   tests. Scenarios find problems in the *relationships* among components — where integration bugs live. Exaggerate
   slightly to increase power ("soap opera tests").
5. **Easy to evaluate** — despite complexity, pass/fail must be clear. Paramount for autonomous verification: if
   pass/fail is ambiguous, an agent will accept plausible-but-wrong output.

Tester's perspective (distinct from a requirements analyst): expose credible concerns; don't have to reach
conclusions, respect prior agreements, or be exhaustive — just useful.

## Workflow

### Phase 1: Understand the System

Gather context (ask, don't assume): what the system does and who uses it; the key benefits it delivers; the
riskiest/most complex areas; known problem areas / past failures; the environment and external interactions;
the existing test suite and its gaps. Read key files (README, entry points, existing tests) if given a codebase.

### Phase 2: Generate Scenarios Using the Twelve Techniques

Each is a different lens. Not all apply — select what fits, use several for breadth. Generate 2-5 candidates per
technique applied.

1. **Life histories for objects** — trace an entity (account, order, device, record) from creation through every
   state change to termination. What could go wrong at each transition? Unusual-but-legitimate transitions?
2. **List users, analyze their interests/objectives** — every user type (not just primary: admins, support,
   auditors, managers), their interests, their objectives. Build scenarios around users pursuing objectives.
3. **Disfavored users — how do they abuse the system?** — adversarial users; their objectives (steal, bypass,
   hide, confuse). *The implementing agent is itself a disfavored user* — see below.
4. **List system events — how are they handled?** — every occurrence the system responds to. Normal handling,
   then: two at once? unexpected state? malformed?
5. **List special events — what accommodations?** — rare-but-expected occurrences (year-end, DST, leap year,
   maintenance, failover, migration). What happens to in-progress work?
6. **List benefits, create end-to-end tasks** — the top benefits the system promises; a scenario obtaining each
   end-to-end from the user's real starting point. The most direct "why does this exist?" → "does it work?"
7. **Interview users about famous failures** — worst things that happened with this/previous systems. (No direct
   access? mine bug reports, support tickets, postmortems, forum complaints, competitor reviews.)
8. **Work alongside users** — observe real work: workarounds, unexpected sequences, undocumented behavior. (Agent:
   analyze usage logs / session recordings / analytics.)
9. **Read what systems like this should do** — domain research; expected capabilities vs. actual behavior.
10. **Study complaints about predecessors/competitors** — take "user errors" seriously; convert into scenarios.
11. **Create a mock business, process its data** — sustained realistic use, one transaction at a time, pushed hard.
12. **Convert real historical data** — run known-good historical data through the system, compare against
    known-good outputs. Watch for silent failures — output that looks plausible but is wrong. (Kaner: 35% of
    field bugs had been exposed by tests but not recognized — check results carefully.)

For each scenario, produce the `michi:scenario-template` block (Technique, Story, What could go wrong, Complexity,
Verification Level, Given-When-Then, Pass/Fail/Trap, Priority, Holdout?, Credibility notes).

### Phase 3: Build the Test Plan

Organize scenarios by priority (Critical / High / Medium / Low = likelihood × impact), with Context, Scope,
Coverage notes (what's well covered, what gaps remain), and Risks/Limitations (what this plan doesn't cover).

### Phase 4: Acceptance Criteria (optional)

Convert selected scenarios into the plan doc's criteria: **Must Pass (blocking)**, **Should Pass (expected)**,
**Edge Cases (if time permits)**, plus an **Evaluation Method** for each.

## Adapting for Autonomous Agent Verification

Three usage patterns — decide during planning which each scenario is:

- **Pattern 1 — scenarios as task spec.** Include as acceptance criteria the implementing agent sees. Improves
  output quality; risk: the agent over-fits to the scenarios.
- **Pattern 2 — scenarios as holdout verification.** Written and withheld from the implementing agent; a separate
  verification process runs them after. **The agent can't game what it can't see** — catches "teaching to the
  test." Requires separating implementing and verifying contexts. Mark these `Holdout? Yes`.
- **Pattern 3 — scenarios as exploration guides.** Use the techniques to discover what tests should exist before
  any code is written; output feeds both task spec and holdout sets. The primary use of this skill.

### The Agent as Disfavored User (technique #3 applied)

The implementing agent abuses the process structurally, not maliciously:

- **Assert true** — tests that always pass regardless of correctness
- **Circular validation** — deriving expectations from the implementation, not the requirements
- **Minimal compliance** — satisfying the literal text while missing intent
- **Pattern matching without understanding** — copying patterns without knowing why
- **Silent scope reduction** — doing the easy parts, skipping the hard parts
- **Plausible-looking output** — reads well, subtle logic error

Design scenarios that specifically catch these. For every scenario, define **pass** (specific, observable),
**fail** (specific, observable), and the **trap** (looks like a pass but is actually a fail — what an agent
accepts).

### Verification Levels

- **A** — automatically verifiable, binary pass/fail (run code, assert, check output). Start here.
- **B** — programmatically checkable with judgment (LLM-as-judge, fuzzy match).
- **C** — requires human judgment (subjective quality, UX, business context).

As confidence grows, promote C → B → A by making criteria more specific.

### Scenario Lifecycle

Generate → Prioritize (risk × automability) → Allocate (task spec vs. holdout) → Implement (Level A → runnable
tests) → Run (verification gates) → Evolve (after a bad output: "what scenario would have caught this?" — add it) →
Rotate (don't reuse forever; create variations; rotate old holdouts out). Store the catalog in
`docs/epics/<epic>/verification/scenarios.md`.

## What This Skill Does NOT Do

Write test code, run tests, or replace other techniques (unit, domain, stress). Scenario testing is one approach —
combine it with the others.

## Scenario template

A scenario is a story about a person using the system for a real purpose — not a
checklist of inputs and outputs. A good scenario (Kaner's five criteria) is: a
story, motivating, credible, complex (multiple features/flows), and easy to
evaluate. Write 3–5 per milestone. Decompose Level A/B into Given-When-Then.

Copy the block below once per scenario.

---

### Scenario: [Short descriptive name]

**Technique**: [Which scenario-generation lens produced this — e.g. object life
history, user types, disfavored user, system events, benefits end-to-end]

**Story**: [2–4 sentences. A real person doing something real: who they are,
what they're trying to accomplish, why it matters, and the consequence of
failure.]

**What could go wrong**: [The failure or unexpected behavior this scenario
exposes — the integration-boundary gap, not a single-feature bug.]

**Complexity**: [Which features, components, or data flows are involved. A
scenario touching one feature belongs in unit tests, not here.]

**Verification Level**:
- **A** — agent executes autonomously, binary pass/fail (API call, data-store
  query, concrete assertion)
- **B** — agent executes, but evaluation needs judgment (error-message clarity,
  output quality) — LLM-as-judge or human
- **C** — human evaluates (UX feel, visual design, workflow naturalness)

**Given-When-Then** (Level A/B — carry state between steps):

```
Given [precondition]
And   [additional precondition]
When  [action]
And   [additional action]
Then  [observable expected outcome]
And   [additional expected outcome]
```

**How to evaluate**: [For Level C, or evaluation notes beyond Given-When-Then.]

**Pass / Fail / Trap**: [Make the result unambiguous — this is what stops an
agent accepting plausible-but-wrong output.]
- **pass**: [specific, observable]
- **fail**: [specific, observable]
- **trap**: [output that looks like a pass but is actually a fail — the subtle
  wrong result]

**Priority**: [Critical / High / Medium / Low — likelihood × impact]

**Holdout?**: [Yes/No. If yes, this scenario is authored by a human and withheld
from the implementing agent's context — the independent gate it can't game.]

**Credibility notes**: [Why a stakeholder with influence believes this will
probably happen in the real world — observed behavior, a spec, competitor usage,
real data patterns.]
