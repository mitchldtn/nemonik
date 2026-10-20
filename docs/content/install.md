---
title: Install
group: Getting started
order: 4
description: Install nemonik with Homebrew or the install script on macOS, the install script on Linux, or build it from source.
---

nemonik is one binary. It needs tmux, which it runs every agent in.

## macOS

Homebrew installs a signed, notarized universal binary (Apple Silicon and Intel), tmux and the `nk` alias.

```sh title="shell"
$ brew install mitchldtn/tap/nemonik
```

### Without Homebrew

The script installs the same signed universal binary and the `nk` alias to `~/.local/bin`. It doesn't install tmux; get it from Homebrew or MacPorts (`sudo port install tmux`). It tells you if tmux is missing.

```sh title="shell"
$ curl -fsSL https://raw.githubusercontent.com/mitchldtn/nemonik/main/scripts/install-macos.sh | sh
```

Set `NEMONIK_VERSION` to pin a version or `NEMONIK_INSTALL_DIR` to install somewhere else. To upgrade, run it again, with the same `NEMONIK_INSTALL_DIR` if you set one.

## Linux

The script installs a static binary (x86_64 or aarch64) and the `nk` alias to `~/.local/bin`. It tells you if tmux is missing.

```sh title="shell"
$ curl -fsSL https://raw.githubusercontent.com/mitchldtn/nemonik/main/scripts/install-linux.sh | sh
```

Set `NEMONIK_VERSION` to pin a version or `NEMONIK_INSTALL_DIR` to install somewhere else. To upgrade, run it again, with the same `NEMONIK_INSTALL_DIR` if you set one. The tarballs are also on the [Releases](https://github.com/mitchldtn/nemonik/releases) page.

HTTPS uses the system's CA certificates. Minimal container images may need `ca-certificates` installed.

## Build from source

```sh title="shell"
$ cargo build --release
$ cargo test --workspace
```

The workspace has two crates: the `nemonik` binary and its library, `nemonik-core`. Use `--workspace` to test both.

:::note
`nk login` on Linux stores its token in GNOME Keyring or KWallet. On a server without one, use `NEMONIK_API_KEY`; see [nk login](/cli-login/).
:::
