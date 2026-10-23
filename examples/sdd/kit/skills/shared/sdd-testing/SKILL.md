---
name: sdd-testing
description: The sdd test budget and which checks to run when: which tests to write, and what to run while building, in local verification and after a fix. Use when deciding whether to add a test, running checks or reviewing tests.
---

# sdd testing

Scenarios are run against the real program; tests cover the logic and seams a scenario can't reach
cheaply. These rules win over a task's wording: skip a test they rule out and log that as a
decision.

## Which tests to write

- Test new logic and the seams between components. Extend an existing test before adding a new one.
- Don't test what can't break (a field passed through unchanged, a constant, frozen data), what only
  checks a mock, a case another test already covers, or a scenario's steps: the scenario run is the
  check.
- Where a test is warranted, write it first and see it fail.

## What to run when

| When | Run |
|---|---|
| Before building | the full suite, once, for the starting state |
| After each change | the tests of the code you changed, not the full suite |
| Local verification | the local scenarios against the real program, then the full suite, once |
| After a fix that changes behavior | the tests of the changed code, the scenarios that cover it, then the full suite once |
| After a fix that changes code but not behavior (a refactor, a removed test) | the tests of the changed code, then the full suite once |
| After a fix to docs or comments only | nothing |
| Reviewing | the program, never the suite; the code reviewer runs nothing |
