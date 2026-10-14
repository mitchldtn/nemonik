---
title: nk trust
group: Reference
order: 4
description: Set or print the trust policy nk run uses, strict or off.
---

Sets the trust policy `nk run` uses, or prints it.

```text title="usage"
nk trust [strict|off]
```

| Policy | Meaning |
|---|---|
| `strict` :tag[default] | A new or changed kit is shown and confirmed before it starts. With no terminal it stops. |
| `off` | No check. `run` starts any kit and prints a warning. |

With no argument, prints the current policy and the order it is resolved in. The setting is saved as `trust` in [config.json](/config/).

:::note
Order: `--trust`, then `NEMONIK_TRUST`, then `config.json`, then `strict`.
:::
