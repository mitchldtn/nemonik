# nemonik

Package how your team works into kits you share and refine: the system prompt,
skills, MCP servers and hooks for planning, coding and reviewing. A running
memory, journal and status carry each project forward across sessions. Claude
or Codex, one command.

[![CI](https://github.com/mitchldtn/nemonik/actions/workflows/ci.yml/badge.svg)](https://github.com/mitchldtn/nemonik/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/mitchldtn/nemonik)](https://github.com/mitchldtn/nemonik/releases/latest)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![Docs](https://img.shields.io/badge/docs-docs.nemonik.io-blue)](https://docs.nemonik.io)

## What it does

The CLI runs standalone with local kits defined in a `nemonik.json`.

## Install

macOS (Homebrew installs nemonik, the `nk` alias and tmux):

```sh
brew install mitchldtn/tap/nemonik
```

macOS without Homebrew (installs the universal binary and the `nk` alias to `~/.local/bin`; tmux comes from Homebrew or MacPorts):

```sh
curl -fsSL https://raw.githubusercontent.com/mitchldtn/nemonik/main/scripts/install-macos.sh | sh
```

Linux (installs a static binary and the `nk` alias to `~/.local/bin`):

```sh
curl -fsSL https://raw.githubusercontent.com/mitchldtn/nemonik/main/scripts/install-linux.sh | sh
```

To upgrade, run the same script again.

More options are on the [Install](https://docs.nemonik.io/install/) page.

## Quick start

Run two local kits, a coder and a reviewer, no account needed:

```sh
nk example coder-reviewer
cd coder-reviewer
nk verify coder
nk verify reviewer
nk run coder "Add input validation to src/login.ts."
```

The coder makes the change and starts the reviewer. They message each other
until the reviewer approves.

nemonik won't run a kit you haven't approved. The reviewer starts in the
background, where it can't ask you, so approve both kits first:

- `nk verify coder` and `nk verify reviewer` show each kit. Type `y` to approve it.
- Or skip approval with `nk trust off`. `nk trust strict` turns it back on.

Run a hosted kit:

```sh
nk login
nk run mitchldtn/code:review --project my-app "review pr 128"
```

## Documentation

Everything else is at [docs.nemonik.io](https://docs.nemonik.io):

- [Quickstart](https://docs.nemonik.io/quickstart/)
- [nk run](https://docs.nemonik.io/cli-run/)
- [Local kits](https://docs.nemonik.io/local-kits/)
- [Kit names and nemonik.json](https://docs.nemonik.io/kit-names/)
- [The context store](https://docs.nemonik.io/context-store/)
- [config.json](https://docs.nemonik.io/config/)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT or Apache-2.0, at your option: [LICENSE-MIT](LICENSE-MIT),
[LICENSE-APACHE](LICENSE-APACHE).
