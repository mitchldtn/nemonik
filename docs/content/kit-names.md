---
title: Kit names and nemonik.json
group: Kits
order: 3
description: How a hosted kit is named, and the project, aliases and kits a repo's nemonik.json sets.
---

How a kit is named, and how a repo sets defaults so nobody repeats them.

## Kit names

A hosted kit's full name is `owner/category:name`, where the owner is a team key or a person's username. The owner is required; a bare `category:name` is rejected.

## nemonik.json

Commit a `nemonik.json` to your repo. `run` and `verify` find it by walking up from the current folder, stopping at a `.git` boundary.

```json title="nemonik.json"
{
  "project": "my-app",
  "aliases": { "review": "mitchldtn/code:review" }
}
```

- **project** is the default when `--project` is absent. It scopes `--remote` and names the session; it doesn't change which kit a name means.
- **aliases** map a short name to a full kit name, so `nk run review` runs `mitchldtn/code:review`.
- **kits** define local kits; see [Local kits](/local-kits/).

:::note
Only `project`, `aliases` and `kits` are read. A `nemonik.json` can't set `--yolo` or the trust policy, since it may come from a repo you just cloned. A malformed file is an error; a missing or empty one is ignored.
:::

:::note
Order for the project: `--project`, `$NEMONIK_PROJECT`, `nemonik.json`.
:::
