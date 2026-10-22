---
title: Local kits
group: Kits
order: 4
description: Define kits from files in your repo under kits in nemonik.json, with examples of two kits that talk and four kits that chain.
---

A kit built from files in your repo instead of fetched from the server. It runs by a short name, needs no login and makes no network call.

## The shape

Local kits live under `kits` in `nemonik.json`. Every part is optional.

```json title="nemonik.json"
{
  "project": "my-app",
  "kits": {
    "mykit": {
      "prompt":          "./prompts/mykit.md",
      "skills":          "./skills",
      "mcps":            "./mcps.json",
      "hooks":           "./hooks.json",
      "mcp_mode":        "extend",
      "launch_args":     ["--model", "fable"],
      "workflow":        "./prompts/mykit-workflow.md",
      "workflow_inject": "user"
    }
  }
}
```

| Field | Points at |
|---|---|
| `prompt`, `workflow` | A `.md` file. |
| `skills` | A folder of `<name>/SKILL.md` skills. Subfolders without a `SKILL.md` are skipped. |
| `mcps`, `hooks` | A JSON file holding an array of specs. |
| `mcp_mode` | `extend` or `only`. Anything else is an error. |
| `launch_args` | Flags appended to the agent's command line. |
| `workflow_inject` | `user`, `system` or `both` (default). |

## MCP and hook files

`mcps` is a JSON array of MCP servers. A `stdio` server needs a `command`; an `http` server needs a `url`. `args`, `env` and `headers` are optional.

```json title="mcps.json"
[
  { "name": "fmt", "type": "stdio", "command": "npx", "args": ["my-mcp"], "env": { "TOKEN": "..." } },
  { "name": "svc", "type": "http", "url": "https://example.com/mcp" }
]
```

`hooks` is a JSON array of hooks. `config` is a Claude Code hooks settings fragment; `description` is optional.

```json title="hooks.json"
[
  { "name": "bash-audit", "config": { "PreToolUse": [ { "matcher": "Bash", "hooks": [ { "type": "command", "command": "echo hi" } ] } ] } }
]
```

## Paths

Every part is a local path written explicitly: `./`, `../`, `/` or `~`, or a bare `.` or `..`. Relative paths resolve against the folder that holds `nemonik.json`, and `~` expands to your home folder. Anything else, such as `prompt.md` or a hosted kit name, is an error.

## Example: a coder and a reviewer

Two local kits that drive each other. The coder makes a change and starts the reviewer; they message each other with `nk write` until the reviewer approves.

```sh title="shell"
$ nk example coder-reviewer
$ cd coder-reviewer
$ nk verify coder
$ nk verify reviewer
$ nk run coder --yolo "Add input validation to src/login.ts."
```

`--yolo` lets each agent run `nk run` and `nk write` without asking you first.

## Example: chaining kits

The `sdd` example is a spec-driven workflow split into four kits: plan, build, review and deep review. Each step runs in its own session, and each kit starts the next one.

```diagram title="flow"
sdd:plan ──(you approve)──> sdd:build <──nk write──> sdd:review, then sdd:deep_review
                                │
              both approve ─> you say ship ─> live checks ─> fold, debrief
```

All four kits share one system prompt. Each has its own skills and its own workflow, put in the first message.

```json title="nemonik.json (two of the four kits)"
{
  "kits": {
    "sdd:plan": {
      "prompt":          "./kit/prompt.md",
      "skills":          ["./kit/skills/shared", "./kit/skills/plan"],
      "workflow":        "./kit/plan.md",
      "workflow_inject": "user"
    },
    "sdd:review": {
      "prompt":          "./kit/prompt.md",
      "skills":          ["./kit/skills/shared", "./kit/skills/review"],
      "workflow":        "./kit/review.md",
      "workflow_inject": "user"
    }
  }
}
```

### Set it up

```sh title="shell"
$ nk example sdd
$ cp -R sdd/nemonik.json sdd/kit sdd/sdd /path/to/your/repo/
$ cd /path/to/your/repo
```

Fill in `sdd/project.md` with your stack, test command and conventions.

### Run a change

```sh title="shell"
$ nk run sdd:plan "add recurring tasks"
```

1. **Plan** reads `sdd/project.md`, your specs and memory, explores the code and asks you questions one at a time. It writes `sdd/changes/add-recurring-tasks/` and asks you to review it.
2. Reply `approved`. Plan starts **build** in a new window, which needs `new_window_command` ([window commands](/config/#window-commands)); without it, build takes over your window. Plan then closes itself.
3. Build works through the tasks test-first, committing on the change branch, and verifies the change locally. It starts **review** in the background, then **deep review** once review approves the code. Build goes back and forth with each over `nk write` until it replies APPROVED. Build shows you each review finding with its recommendation and fixes only what you approve. Nothing is pushed or deployed yet.
4. Build asks you before it pushes, opens PRs or deploys. After a deploy it runs the scenarios that need the deployed environment and sends the evidence to review for its final verdict.
5. Build updates `sdd/specs/`, archives the change and debriefs: what was learned, what the change made untrue, and `sdd/memory/`. It commits those files on the change branch and pushes them with your go.

Add `--agent codex` to the first command to run the whole chain on Codex; each kit passes it on.

### How the kits hand off

The handoffs are ordinary nemonik commands in each kit's skills. Plan starts build in a new window, and build starts each reviewer in the background and tells it where to reply:

```sh title="run by the agents"
$ nk run sdd:build --new -- add-recurring-tasks
$ nk run sdd:review --no-attach --yolo -- Review change add-recurring-tasks. Reply to session <build session>. Deployed later: S2.
$ nk run sdd:deep_review --no-attach --yolo -- Deep review change add-recurring-tasks. Reply to session <build session>.
```

Each kit's workflow sets the order of its steps and names the next kit. To change the order, for example to skip the deep review, edit `kit/build.md`.

:::callout[Note]
Review never sees how build reasoned, only the change itself. That is why it runs in its own session.
:::

## Trust

A local kit goes through the same check as a hosted one. Editing any of its files changes the hash, so the next `run` asks again.

The review masks MCP `env` and `headers` values, but a local kit reads them as written from the repo's `mcps.json` and passes them to the MCP server. `verify` and `run` warn about this; read the file before trusting a kit from someone else's repo.

:::note
`sdd-nemonik` is the same four kits with every document in the nemonik context store instead of files. It needs a nemonik account and a `project` in `nemonik.json`.
:::

:::note
Both reviews run with `--yolo`, so they run your tests without permission prompts.
:::

:::note
A name under `kits` wins over an alias with the same name.
:::

:::note
`skills`, `mcps`, `hooks` and `launch_args` accept a single string or an array.
:::
