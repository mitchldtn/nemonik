---
title: nk verify
group: Reference
order: 2
description: Fetch a kit, show its exact content, and pin its hash when you approve.
---

Fetches a kit, shows its exact content, and pins it when you approve.

```text title="usage"
nk verify [--web] <KIT>
```

## The kit argument

A full kit name, `owner/category:name` (for example `mitchldtn/code:review`), or an alias or local kit name from [nemonik.json](/kit-names/).

## --web

Also opens the content in your browser, with a diff against the pinned version when it changed.

## Answering

Type `y` or `yes` and Enter to pin. Anything else, or an empty line, leaves it unpinned. An unchanged kit prints `Already verified (unchanged)`.

`verify` needs a terminal: with piped or redirected input it stops before showing anything, and pins nothing.

## Optional before run

`nk run` does the same review the first time it meets a new or changed kit. Run `verify` on its own to pin ahead of time, for example before a run with no terminal.

:::note
See [Trust and verification](/trust/) for what the pin covers.
:::
