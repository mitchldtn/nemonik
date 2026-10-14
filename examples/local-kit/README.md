# One local kit that uses a prompt, a skill, a hook and a workflow

A local kit is assembled from files next to a `nemonik.json` instead of being
fetched from nemonik, so nemonik assembles it without a login or a network call.
This one is a repository guide: started in a repo, it gives a short tour and
then answers questions about the code.

## Run it

```bash
nemonik example local-kit
cd local-kit
nemonik verify local-kit   # review the assembled kit and press y to pin it
nemonik run local-kit
```

Started here, it tours this folder. To use it on your own project, copy
`nemonik.json` and the files it points to into that repo and run it from there.
`nemonik.json` is found by walking up from the current directory.

## The files

| File | Kit part | What it does |
| --- | --- | --- |
| `nemonik.json` | | Declares the kit `local-kit` and points at each part below. Paths start with `./` and are relative to this file. |
| `prompt.md` | `prompt` | The system prompt: who the agent is and how it should answer. |
| `skills/repo-tour/SKILL.md` | `skills` | A skill the agent can use. Every `<name>/SKILL.md` under `skills/` becomes a skill named after its folder. |
| `hooks.json` | `hooks` | A JSON array of hooks. This one is a Claude Code `UserPromptSubmit` hook that adds a reminder to every prompt. |
| `workflow.md` | `workflow` | Instructions injected into the session. `workflow_inject: "user"` puts them in the first user turn, which is what makes the agent start with the tour. |

Hooks are applied on Claude Code only. On `--agent codex` nemonik warns and
launches without them.

## Change it

Edit any file and run `nemonik verify local-kit` again: the kit's content
changed, so nemonik shows it to you before the next run. A kit can also attach
MCP servers with `"mcps": "./mcps.json"`, a JSON array of server specs.
