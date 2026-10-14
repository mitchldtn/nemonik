---
title: Linked docs
group: Context
order: 3
description: Linking context docs to tickets, PRs, branches and commits when you wrap up work.
---

A doc's links are stated, never read out of its body. GitHub, GitLab and Jira links are also recognized as tickets and PRs.

## Wrapping up work

Commit, push, open the PR, then `put` the context last, so the PR exists when you link it.

```mcp title="MCP"
put(project="my-app", slug="dev-52-context", ticket="DEV-52", body="...",
    branch="DEV-52-feature", commit_sha="<git rev-parse HEAD>",
    links=["<ticket url>", "<pr url>"])
```

:::note
Context never blocks a push, and a push never requires context.
:::
