# Michi Session

Execute an implementation milestone. This skill is **rigid** — follow every step. The discipline is the point.

**Violating the letter of this process is violating its spirit.** "I'm following the intent, just adapting the
steps" is a rationalization. The steps ARE the intent.

## The Iron Law

```
NO "DONE" WITHOUT RUNNING FULL VERIFICATION
```

If you haven't executed the verification checklist in this session, you cannot claim the milestone is complete.
"I believe it passes" is not verification. Run it.

## Report Register

The plan doc is the report; chat is the summary. Write progress detail — steps completed, decision logs,
deviations — into the plan doc as it happens. Chat output per turn is small: what changed, anything surprising,
what's next, and any question (self-contained, ending the turn). Reply size follows the ask. The verification
discipline is non-negotiable; the *narration* of it is the human's dial.

## Target Deliverable

- **Code** — working software, tests, runnable code. Core loop below (Implement → Test → Repeat) applies.
- **Non-code** — artifacts, recommendations, research, design docs. Core loop is Explore → Synthesize →
  Checkpoint; verification uses exit criteria instead of scenarios. The rest of this skill applies regardless.

## Pre-Flight

1. **Read the milestone plan.** `docs/epics/<epic>/plans/<t>-plan.md` (or find it with `ls docs/epics/*/plans`). This is your contract.
   If it doesn't exist, stop and use `michi-planning` first.
2. **Verify branch.** `git branch` — confirm the correct feature branch.
3. **Verify environment.** Code: services running, seed data loaded, suite passes from clean state. Non-code:
   source materials accessible.
4. **Read project conventions.** The project's CLAUDE.md / referenced docs; and its memory doc
   (`memory.md`) if it exists — how we work together, active threads, landmarks.
5. **Look for open source.** Scan the plan's steps for commodity functionality (parsing, HTTP, dates, I/O,
   validation). Prefer an existing library over custom code; note findings in the plan so the core loop doesn't
   stop for research.

## The Core Loop

**Code — TDD by default:**

```
Write failing test → Implement → Pass → Refactor → Repeat
```

- Write the failing test before the implementation. Red first, then green.
- Run the full suite after every file change (not every step — every file). The fast loop is the primary gate.
- If tests fail (other than the one you just wrote), fix before moving on. Don't accumulate broken tests.
- Refactor only on green.
- Skip TDD only when the human explicitly says so (spikes, prototypes, mechanical refactors pinned by existing tests).

**Non-code:** Explore → Synthesize → Checkpoint against exit criteria.

**Prefer existing libraries over custom code.** **Don't optimize without justification** — the default is the
straightforward approach; optimization must be defended. **The Rule of 3:** don't stop before three rounds on a
hard problem; don't push past three when rounds stop producing progress.

### Evidence Before Claims

If you state that something works, passes, or is complete, you must have run the verification in the current
context. "Should work" / "I believe it passes" are not evidence. Run the command, read the output, then state
the result. Applies to test results, API calls, scenario execution, and scope assertions.

### Reactive Scope Changes (agent-initiated)

Implementation sometimes reveals a gap the plan didn't anticipate. **In Paired mode**, a reactive scope change is
legitimate when: a spike confirms feasibility, the human agrees, and it's logged as a decision (`design-choice`
or `contract-change`). This is not scope creep — the human's agreement is the gate. **In Entrusted mode**, prefer
a new milestone over extending the current one; the plan is the contract.

### Human-Initiated Scope Changes

- **End-loaded growth** (fits after current milestones, doesn't reshape them) — absorb silently and keep working.
- **Disruption** (reshapes planned milestones, breaks scope contracts) — surface, confirm, then proceed. Replan
  if needed.

Default to absorb.

## Decision Logging

When you make a choice without human input, log it immediately in the plan doc's `## Decisions` section (edit the
plan file in place):

```markdown
### [Short title]
- **Decision:** [What you chose]
- **Alternatives:** [What else you considered]
- **Reasoning:** [Why this choice]
- **Impact:** implementation-detail | design-choice | contract-change
- **Reversible:** Yes / No
```

Classification: `implementation-detail` — log it, no notification. `design-choice` — log it, human reviews at
debrief. `contract-change` (public API, schema, new service type) — **log it AND notify the human immediately.**
If unsure whether something is a contract-change: it is. Notify.

## Discussion Items

Things where you don't have enough context to decide alone but aren't blocked. Log them in `## Discussion` as they
arise (e.g. "mocking strategy feels inconsistent across packages — align?"). Reviewed during debrief.

## Red Flags — STOP

If you catch yourself thinking any of these, stop and follow the process:

- "This decision is too minor to log" — Log it. The debrief decides what matters.
- "Tests passed, verification is redundant" — Post-milestone verification catches what unit tests miss. Run everything.
- "I'll update the plan doc / status later" — Do it now. Later means never.
- "This scope change is small enough to not need logging" — If it changes what's delivered, log and classify.
- "I already know the branch is correct" — Run `git branch`. Every time.
- "I can skip pre-flight between milestones" — Pre-flight resets. Always re-verify.

## Post-Milestone Verification

After the plan's steps are complete, run the full verification. **Every item is mandatory.** Evidence, not
assertions.

1. **Self-review** — re-read the diff (`git diff HEAD`) against acceptance criteria. Look for hardcoded caller
   assumptions, missing schema/enum updates, URL-fragment concatenation bugs, unused imports / dead copy-paste code.
2. **Execute Level A scenarios** — run each Given-When-Then from the plan. Report pass/fail per step.
3. **Unit tests pass.**
4. **Integration tests pass.**
5. **Cross-package schemas updated** — if new types, enum values, or service names were introduced, verify all
   downstream validation schemas.
6. **No hardcoded caller assumptions** — grep for hardcoded source/service/caller strings in shared code.
7. **Scope check** — compare the diff against the plan's `## Scope`. If implementation expanded, log as a decision.

If any item fails, fix before proceeding.

### Code Review (independent subagent — this is "the gap")

For milestones touching multiple packages, introducing new types/callers, or modifying schemas, launch a
code-reviewer subagent:

- **Provide:** the diff, the plan doc, the acceptance criteria.
- **Do NOT provide:** your implementation reasoning or decision context — the reviewer evaluates independently.
  This independence is what makes the review catch what you can't.
- **The reviewer checks for:** bugs, logic errors, security issues, missed edge cases, convention violations.

For single-package, low-risk milestones, self-review is sufficient — but err toward reviewing. Address
high-confidence issues before proceeding.

### Record Verification in the Plan

Fill the plan doc's `## Verification` section with the per-layer results (which layers ran, pass/of counts, what
each caught, the code-review outcome) in the plan file. Do not declare "done" until every mandatory item passes.

### Sustainability Check (pre-commit)

Brief review scaled to the milestone. **Refactoring:** obvious duplication or unclear naming in the work just done?
Copy-paste that should be extracted? Names revealing intent? **Tests:** mocks at I/O boundaries only? Test names
describe behavior, not implementation? Assert on outcomes, not internal calls? Obvious missing cases? Add
anything that emerged to `## Discussion`.

## Ready to Commit

Hand off to the human with a proposed commit message. Git policy: the agent doesn't run mutations; the human
commits.

## Post-Milestone

1. **Update the plan doc** — check off completed steps, note deviations.
2. **Update the project status doc** — read `STATUS.md` cold, edit anything stale, and update its
   `**Last updated:**` stamp. Required, not conditional.
3. **Notify** — what was completed, results (test counts / exit-criteria assessment), any contract-change decisions
   needing review, whether you're continuing or stopping.

## When You're Stuck

Re-read the relevant material. Check if the error message contains the fix. If genuinely blocked: document the
blocker in the plan doc, notify the human, and wait — don't work around blockers silently. If you've attempted the
same fix 3+ times, stop: log the pattern in `## Discussion`, describe what you tried and why it keeps failing, and
escalate. Three failed fixes is a signal to question fundamentals, not try a fourth.

## Session End

- Verify all milestones are committed.
- Confirm the plan doc and status doc are current.
- **Memory check:** "What from this session would be painful to lose in a new session?" If warranted, update the
  project memory doc (`memory.md`) — collaboration patterns, corrections, confirmed
  approaches, new landmarks. Not implementation details (those belong in the plan doc or journal).
- The natural next step is `/michi-debrief`.
