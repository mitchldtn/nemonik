---
name: sdd-scenarios
description: How to write sdd verification scenarios and holdouts (story-based, Level A/B/C, pass/fail/trap) and the coverage table that ties every changed requirement to a scenario or test.
---

# sdd scenarios

A scenario is a story about a person using the system for a real purpose, broken into
Given/When/Then steps that can be run. Spec scenarios (the WHEN/THEN lines inside requirements)
say what one behavior is. Verification scenarios check that behaviors work together, which is
where integration bugs live.

## What makes a good one

1. **A story:** someone doing something for a reason, and what it costs them if it fails.
2. **Motivating:** someone who matters would push to fix it.
3. **Credible:** it will probably happen, not just could.
4. **Complex:** it crosses several features or components. A one-feature check is a unit test.
5. **Easy to evaluate:** pass and fail are unambiguous. If they aren't, an agent will accept a
   plausible wrong result.

## Where to find them

Use the lenses that fit; two or three are usually enough.

- **Life of an object:** follow one record from creation through every state change to deletion.
- **Users and goals:** each kind of user, what they are trying to get done.
- **Abusers:** who misuses this, and how.
- **Events:** everything the system reacts to; two at once, out of order, malformed.
- **Special moments:** month end, a clock change, a migration, a restart mid-operation.
- **Benefits end to end:** each promised benefit, from the user's real starting point.
- **Past failures:** bug reports and complaints, here or in similar tools.

## Levels

- **A:** the agent runs it and the result is binary (a command's output, a file's contents, a test).
- **B:** the agent runs it but judging needs care (is the error message clear?).
- **C:** the human judges (does the workflow feel right?).

Prefer A. A scenario that can't be made A or B is often a sign the behavior isn't defined well enough
yet; say so in the proposal's open questions.

## Environment

Each Level A scenario says where it runs: `local` (the test suite, a local server, a scratch
database) or `deployed (<environment>)` (it needs a real environment, such as a staging cluster).
Build runs local scenarios before review, and deployed ones only after both reviews approve and the
human has deployed. Prefer local; make a scenario deployed only when a local run can't show what it
checks.

## Pass, fail, trap

Every scenario names a specific pass, a specific fail, and the **trap**: a result that looks like a
pass but isn't. Example: "the export file exists" is a trap when it is empty.

## Holdouts

Holdouts are scenarios build never sees; review runs them. Aim them at the shortcuts an implementer
takes:

- tests that always pass
- expectations copied from the code instead of the spec
- doing the easy part and quietly skipping the hard part
- output that reads well but is subtly wrong

Two or three per change are enough. They are written by this planning session, not by the human, so
tell the human they may add their own to the `<id>-holdouts` document before approving.

## Coverage

Every ADDED or MODIFIED requirement gets a row in the coverage table, covered by a scenario or a named
test. A requirement nothing covers is a requirement nobody will check.

Write three to five scenarios for a tier M change; more only when the change is large.
