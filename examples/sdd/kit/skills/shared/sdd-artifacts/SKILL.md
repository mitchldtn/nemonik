---
name: sdd-artifacts
description: Where every sdd file lives under sdd/ and how changes and capabilities are named. Use before reading or writing anything under sdd/.
---

# sdd artifacts

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
│   ├── review.md                       written by build from the reviewers' verdicts
│   └── code-review.md                  written by the code reviewer
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
