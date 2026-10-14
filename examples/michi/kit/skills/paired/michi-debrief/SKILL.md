# Michi Debrief

Run after a michi session completes. Can run in the same session (benefits from context) or fresh (clean context,
no compaction bias). For long sessions, prefer fresh.

**Principles served:** Sustain the system (knowledge capture + currency). Iterate spirally (each debrief improves
the next iteration). Surface assumptions (what got invalidated). Verification governs autonomy (trust calibration).

## Scaling

Match depth to the work. **Full debrief** (all three passes) — epic milestones, the default; produces a debrief
artifact plus extracted findings in their long-lived homes. **Short debrief** — freeform/workshop/small sessions;
a summary journal entry. Pass 1 light, Passes 2 and 3 still required (staleness doesn't care about session size).
**Skip** — trivial changes where the commit message is the whole story. When in doubt, ask the human.

The artifact carries the assessment; chat carries the summary. Store the full debrief:
`docs/epics/<epic>/debriefs/<t>-debrief.md`. Report the two or three findings
that change what happens next; don't restate the artifact in chat.

## The Three Passes

Rule of 3 applied to the debrief. Three passes, named, all required:

1. **Capture** — what happened, what was decided, what was learned. Additive.
2. **Invalidate** — what did this milestone *break*? Docs that now lie, decisions overruled, memory that aged out.
   The pass that closes the doc-drift gap.
3. **Currency + Cleanup** — project docs read cold, archive candidates surfaced. Keeps project state honest.

Then trust calibration, then outputs. Passes can be brief, but each must run.

---

## Pass 1: Capture

### 1.1 Delivery Assessment

For each milestone: scope in the plan vs. what was delivered vs. what was deferred/cut (why). Were acceptance
criteria met? Check each explicitly. Results — code: unit/integration/Level A pass-fail counts, code review
done/skipped/issues. Non-code: exit criteria met/not/partial, deliverables produced/missing. Commits:
`git log --oneline <branch>`. All committed?

### 1.2 Decision and Discussion Review

Read `## Decisions` and `## Discussion` from each plan doc (`docs/epics/<epic>/plans/<t>-plan.md`). For each
decision: reasonable given context? Unanticipated consequences? Reverse/refine/accept? Repeating pattern (codify
it)? Flag decisions that should become standing rules. For each discussion item: resolve now / defer (note why
and when) / promote to project-level.

### 1.3 Bug and Gap Analysis

For each bug found: how was it found (unit/integration/smoke/manual/post-completion), root cause, is the class
systemic? Would a Level A scenario or a holdout have caught it? Would the code-review subagent have caught it?
**Error analysis — evolve the scenario set:** for each bug, write the Given-When-Then scenario that would have
caught it and add it to the catalog (`docs/epics/<epic>/verification/scenarios.md`). This is how verification evolves — driven by real
failures, not imagined ones. Check cross-package gaps: downstream schemas updated? hardcoded caller assumptions?

### 1.4 Process Observations

What worked / didn't (friction, rigid-when-should-adapt, adapted-when-should-be-rigid). Scenario quality: did
Level A catch anything unit tests missed? Too granular / too vague? Executed faithfully? Any B/C to promote to A?
**Scenario lifecycle:** Promote (new, ongoing regression value) / Update (broken by intentional change) / Retire
(no longer relevant) / Add (from error analysis). Human interactions: how many, which were avoidable with better
planning, which were high-value human-only interventions.

### 1.5 Knowledge Capture

- **Domain learnings** (product/codebase gotchas, quirks) → the epic's journal:
  `docs/epics/<epic>/journal.md`.
- **Process learnings** (methodology) → a patterns doc/skill.
- **Applied coding principles** — where the human corrected code structure/naming/organization or pushed back on
  code you considered fine: capture the concrete applied example (trigger, when it applies, what the result looks
  like) → a code-style doc (`docs/code-style.md`).
- **Memory — new entries.** Litmus: "What would be painful to lose in a new session?" Corrections, confirmed
  non-obvious approaches, surprises, unresolved active threads, trust patterns → `memory.md`.
  Boundary: code patterns → code-style; what happened → journal; how we work → memory.

---

## Pass 2: Invalidate

What did this milestone *break*? Additive capture isn't enough — docs that lie quietly never get caught otherwise.
Short when the milestone is small; never optional.

### 2.1 Reference/Doc Audit

For every mechanism this milestone changed (config, API, build step, setup, env convention, migration mechanism),
search the docs: `grep -rn "<old-mechanism>" *.md docs/`. For each hit: **Update** (now wrong —
fix), **Remove** (no longer applies), or **Leave** (historical, correct in context).

### 2.2 Architecture Check

If the milestone touched architecture-relevant code (new components, changed boundaries, new contracts, deleted
modules), read the affected architecture doc sections cold. Still describe reality? Yes → note re-verified today.
No → fix now (or note for Pass 3). Structurally outdated → flag for a focused architecture update.

### 2.3 Decisions/Patterns Superseded

Read the project's key-decisions and patterns docs. Did this milestone change a recorded decision (update/annotate)?
Validate or invalidate a pattern (note the evidence)? Any entries referencing now-removed code/files/tools?

### 2.4 Memory Retirement

Read the project memory doc with the session's changes in mind. Active threads now resolved? Move to landmarks or
remove. Landmarks naming files/functions/tools that no longer exist? Update or retire. Mental-model entries
describing shifted thinking? Refresh or remove. **Stale memory misleads worse than missing memory.**

---

## Pass 3: Currency + Cleanup

### 3.1 Status doc — read cold (mandatory)

Open `STATUS.md` and read it as if new to the project. Every line: still true after this session?
If not, rewrite/remove. Update the active section (what shipped, what's next, what's deferred) and the
`Last updated:` stamp. Not conditional on "anything significant changed" — the reflex is read-cold-and-update.

### 3.2 Journal — timestamp

Ensure the new journal entry has a date heading (`### YYYY-MM-DD — <topic>`) and the doc's `Last updated:` stamp
is current.

### 3.3 CLAUDE/PROJECT/ARCHITECTURE

Read cold *if* this milestone might have affected them (new convention/do-not rule; scope shift; architecture
already covered in 2.2). If unaffected, skip.

### 3.4 Archive Candidates

Surface (don't necessarily act): completed-milestone plan docs, landed sidebars, superseded old debriefs. The
human decides what moves.

### 3.5 Inputs-vs-Outputs Check

Scan what was read (plan Decisions/Notes/Discussion/Scenarios, commits, transcript). Verify each significant
source is reflected somewhere in the output chain (debrief, journal, patterns, rules, memory). Thin outputs from
a thick plan means dropped material — use inputs as a checklist.

---

## Trust Calibration

Assess readiness for the next session's autonomy level.

**Trust increased:** all AC met without intervention; decisions reasonable and documented; no post-completion bugs;
verification checklist followed completely; Level A scenarios passed and caught at least one issue; all three
passes ran cleanly without prodding.

**Trust decreased:** "done" declared prematurely; cross-package gaps found; verification steps skipped/treated as
optional; scenario execution shortcut; decisions that should have been escalated; Pass 2 surfaced doc drift the
agent introduced and didn't catch; Pass 3 found the status doc stale within the same session.

**Recommendation for next session:** same autonomy level / increase / decrease. (In nemonik this reading is the
human-facing input to the paired → entrusted mode choice.)

## Outputs

1. Debrief artifact → `docs/epics/<epic>/debriefs/<t>-debrief.md`
2. Journal entry → `docs/epics/<epic>/journal.md` (date heading, refreshed stamp)
3. Pattern updates → patterns doc/skill (high confidence, clear source only)
4. Standing-rule updates → the project's CLAUDE/conventions doc (durable, broadly applicable only)
5. Status update → `STATUS.md` (required, every debrief)
6. Memory update → `memory.md` (additions from 1.5, retirements from 2.4)
7. Reference-doc edits → wherever Pass 2 found drift
8. Next-session prep notes (optional) — holdout tests to write, environment changes, plan docs to create

### What's Next

If another milestone is ready → `/michi-planning`. If several accumulated without a health check →
`/michi-sustainability`. If the epic is complete → update the status doc and archive.
