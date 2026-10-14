# OpenSpec

This project uses OpenSpec (https://github.com/Fission-AI/OpenSpec): you and the human agree on what
to build before any code is written.

- `openspec/specs/<capability>/spec.md` describe how the system behaves now.
- `openspec/changes/<name>/` holds one proposed change: a proposal, delta specs (ADDED, MODIFIED,
  REMOVED requirements), an optional design and a task list.
- Archiving a finished change merges its delta specs into `openspec/specs/`.

Work through the `openspec-*` skills; they drive the `openspec` CLI and its JSON output, which knows
the artifact order, the instructions for each artifact and the project's rules. Follow the instruction
each command returns rather than writing artifacts from memory.

- Never run `openspec init` and never create `openspec/` files by hand unless the human asks.
- `openspec-propose` plans only: no code edits until the human starts apply.
- Read a file before describing it, and run a check before saying something passes.
- You never commit or push; when work is ready, propose a commit message.
