---
title: nk example
group: Reference
order: 7
description: List the example kits, or write one into a new folder.
---

Lists the example kits, or writes one into a new folder.

```text title="usage"
nk example [NAME] [DIR]
```

| Example | What it shows |
|---|---|
| `local-kit` | One local kit with a system prompt, a skill, a hook and a workflow. |
| `coder-reviewer` | Two kits, a coder and a reviewer, that message each other until the reviewer approves. |
| `sdd`, `sdd-nemonik` | Plan, build and review kits that keep living specs, in local files (`sdd`) or the nemonik context store (`sdd-nemonik`). Build debriefs after you commit. See [Chaining kits](/local-kits/#example-chaining-kits). |
| `michi` | The [michi](https://github.com/csepulv/michi) paired kit and a background reviewer, keeping plans and memory in local files. |
| `openspec` | [OpenSpec](https://github.com/Fission-AI/OpenSpec) as a local kit, on Claude Code or Codex. |

The folder (default `./<name>`) must be missing or empty; nothing is overwritten. No login needed.

:::note
Each example has its own `README.md` explaining its files.
:::

:::note
Examples are read from `examples/` on the `main` branch of the CLI repository. Set `GITHUB_TOKEN` to lift GitHub's rate limit on anonymous downloads; the token is sent only to `api.github.com`.
:::
