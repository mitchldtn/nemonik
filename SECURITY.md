# Security Policy

## Reporting a vulnerability

Please report security issues privately through GitHub's "Report a
vulnerability" advisory flow on this repository rather than opening a public
issue.

## Trust model

nemonik injects a kit (a system prompt plus skills, MCP servers, and hooks)
into a coding agent. Nothing reaches the agent without passing the trust gate:

- `verify` shows a kit's exact resolved content and pins its SHA-256 hash.
- Under the default `strict` policy, `run` re-hashes the kit and refuses to
  launch if it is unpinned or has changed since you reviewed it; a
  non-interactive `run` or `verify` fails closed.
- `verified.json` and `config.json` (which holds the trust policy) are written
  `0600`, and pinned bundles have MCP secrets redacted.

## Remote control (`--remote`)

Remote control is off unless you start it for a session (`nemonik run --remote`
or `nemonik remote`). Only the account that started it can write to, read or
watch the session, which nemonik's server checks on every call. This is remote
code execution over that authenticated channel. Every command the session
receives is appended to `~/.nemonik/harness-daemon.log` as a local audit trail.
