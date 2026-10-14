# OpenSpec as a local kit, on Claude Code or Codex

[OpenSpec](https://github.com/Fission-AI/OpenSpec) is a spec-driven workflow: living specs describe
how your system behaves, each change is a folder with a proposal, delta specs and tasks, and archiving
merges the change back into the specs. This example is one local kit that starts every session with
OpenSpec's rules and a summary of your specs and changes. The skills come from `openspec init`.

## The kit

| Kit | Does |
|---|---|
| `openspec:workflow` | Checks the CLI and the `openspec/` root, summarizes your specs and changes in progress, then uses OpenSpec's skills for what you ask. |

```
nk run openspec:workflow
  /openspec-explore          think it through first (optional, writes nothing)
  /openspec-propose <name>   proposal, specs/, design, tasks; then stop
  /openspec-apply-change     build it, ticking tasks.md
  /openspec-archive-change   merge delta specs into openspec/specs/
```

## Set up

Install the OpenSpec CLI:

```bash
brew install openspec            # or: npm install -g @fission-ai/openspec
```

Then, in your repo:

```bash
nemonik example openspec
cp -R openspec/nemonik.json openspec/kit /path/to/your/repo/
cd /path/to/your/repo
openspec init --tools claude     # or: --tools codex, or --tools claude,codex
nk verify openspec:workflow
```

`openspec init` creates `openspec/` and writes OpenSpec's skills to `.claude/skills/` (Claude Code)
or `.agents/skills/` (Codex), where the agent loads them. On Claude Code it also adds OpenSpec's
`/opsx:*` commands, which do the same work as the skills. `openspec config profile` picks which
OpenSpec workflows you get; run `openspec update` after changing it.

## Run it

```bash
nk run openspec:workflow
nk run openspec:workflow -- propose add-dark-mode
nk run openspec:workflow --agent codex
```

On Claude Code the skills are invoked as `/openspec-propose` and so on; on Codex as
`$openspec-propose`.

## The files in this example

| File | What it is |
|---|---|
| `nemonik.json` | The kit. |
| `kit/prompt.md` | A short system prompt: what OpenSpec is and the rules the skills rely on. |
| `kit/workflow.md` | The first steps every session takes. |

## Limits

- **No nemonik store version.** See `sdd-nemonik`.

## Credits

This kit runs [OpenSpec](https://github.com/Fission-AI/OpenSpec), an MIT-licensed project by OpenSpec
Contributors. The skills it uses are the ones `openspec init` writes.
