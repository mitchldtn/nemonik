---
title: Authoring
group: Kits
order: 2
description: Read, create and edit kits, prompts, skills, MCP servers and hooks with the get, create and edit MCP tools.
---

Kits and their parts are written with the same MCP tools: `get` reads, `create` makes, `edit` changes.

## Read, create, edit

`get` and `edit` take an owner-qualified name: `owner/category:name` for kits and prompts, `owner/name` for skills, MCP servers and hooks. `create` takes `team` (optional), `category` and `name` as separate fields.

```mcp title="MCP"
# read
get(type="kit", name="mitchldtn/code:task")
get(type="prompt", category="pr")        # lists the variants

# create (personal unless team= is given)
create(type="kit", category="code", name="task", prompt={"content": "..."})
create(type="mcp", name="sentry", transport="http", url="https://mcp.sentry.dev/mcp")

# edit; new_name renames
edit(type="kit", name="mitchldtn/code:task", prompt={"content": "..."})
```

## By type

| Type | Name | Created with |
|---|---|---|
| `kit` | `owner/category:name` | `category`, `prompt` |
| `prompt` | `owner/category:name` | `category`, `definition` |
| `skill` | `owner/name` | `body`, `description` |
| `mcp` | `owner/name` | `transport` with `command`/`args` or `url`/`headers` |
| `hook` | `owner/name` | `config`, `description` |

## Embedding prompts

A prompt can include another with `{{ owner/category:name }}`, expanded when it is fetched. Note that `edit` replaces the whole content, so edit from the stored text, not from an expanded `get`.

## Editing a kit

`edit(type="kit")` changes the kit's linked prompt and its name. Attaching or removing skills, MCP servers and hooks, and setting the workflow, `mcp_mode` and `launch_args`, are done in the nemonik app.

:::note
Writing requires editor or owner rank on the owning team.
:::

:::note
MCP `env` secrets are never accepted in plain text over MCP; they are set through nemonik's API. `headers` are accepted and masked when shown.
:::
