# Contributing

nemonik is a Cargo workspace: the `nemonik` binary and its `nemonik-core`
library.

## Build and test

    cargo build --release
    cargo test --workspace     # the test gate across both crates
    cargo clippy --workspace --all-targets
    cargo fmt --all            # format before opening a PR

A bare `cargo test` (without `--workspace`) tests only the binary, so always
pass `--workspace`. The repo is kept rustfmt-clean; run
`cargo fmt --all` before you open a PR. CI runs format, clippy, and tests on
every push and pull request.

## Layout

- `src/`: the `nemonik` binary. `main.rs` (CLI), `launch.rs` (the `run`
  pipeline), `assemble.rs` (local kits from `nemonik.json`), `trust.rs`
  (verify + the trust gate), `review_view.rs` (`verify --web`),
  `session.rs` (tmux sessions), `remote/` (remote control).
- `nemonik-core/`: library. Kit resolution, config, auth, MCP client, skill
  materialization, tmux primitives.

## Pull requests

Keep changes focused. Run the build, test, clippy, and format commands above
before opening a PR. Update `docs/content/` (and the README where it covers the
same thing) when behavior changes, and this file when the developer workflow
changes.
