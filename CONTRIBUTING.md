# Contributing to Pairee

Thank you for helping us make Pairee better!

All activity in this repository is subject to our [Code of Conduct](./CODE_OF_CONDUCT.md).

## How to Contribute

We love pull requests that are:

- Fixing or extending the documentation (e.g. in `docs/`).
- Fixing bugs.
- Making improvements to existing features to support more platforms or configurations.
- Adding small improvements such as keybindings, custom actions, or missing configuration settings.

## Development Setup

Pairee is built using **Rust**. Ensure you have the latest stable Rust toolchain installed.

### Build and Run

To run the application locally:

```sh
cargo run
```

To compile a release build:

```sh
cargo build --release
```

`cargo xtask dev` opens a menu with the everyday developer actions (run,
standalone run, build, release build, tests, check, clippy, fmt, lint,
clean, the WinGet install/upgrade/uninstall commands, local MSIX packaging,
signing and install, and the release). Each entry also runs directly:
`cargo xtask dev clippy`; `cargo xtask dev list` lists them.

Our custom build script `build.rs` automatically captures target compilation, git commit hash, and profile info so you don't get "unknown" values in the About dialog.

### Formatting & Lints

Install the git hook once so every `git push` runs the fast checks (rustfmt,
[typos](https://github.com/crate-ci/typos), translations and help pages; Python 3.11+) first:

```sh
cargo xtask install-hooks
```

Run them by hand with `cargo xtask lint`. Before submitting code, you must ensure that formatting and lints pass:

1. **Format Code**: Check and write formatting.
   ```sh
   cargo fmt --all -- --check
   ```
2. **Clippy Lints**: Run clippy check.
   ```sh
   cargo clippy --workspace --all-targets -- -D warnings
   ```
3. **Unit Tests**: Run tests.
   ```sh
   cargo test
   ```
4. **Dependency policy** (optional locally; required in CI): licenses, advisories, and crate sources.
   ```sh
   cargo deny check
   ```
5. **Fuzzing** (optional; nightly, Linux or macOS): the parsers of untrusted input
   (archives, keymaps, command line, ANSI output, multi-rename patterns, release
   checksums) have [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) targets in
   `fuzz/`. They call `src/fuzz_api.rs`, which `src/test_fuzz.rs` also drives with
   junk input on stable in every test run.
   ```sh
   cargo +nightly fuzz list
   cargo +nightly fuzz run archive_zip -- -max_total_time=60
   ```

### SFTP tests against a real SSH server (opt-in)

The other remote backends are tested against in-process servers; SFTP also
has a suite against a real `sshd`: the Vfs contract and resumed uploads and
downloads. It is compiled in only with `PAIREE_SFTP_TEST=1`:

```sh
docker compose -f docker/docker-compose.yml up -d --build   # sshd on 127.0.0.1:2222
PAIREE_SFTP_TEST=1 cargo test --lib sftp_server
docker compose -f docker/docker-compose.yml down
```

It logs in as `pairee` / `pairee` and works in its own folders below `/tmp`
on the server. `PAIREE_SFTP_HOST`, `PAIREE_SFTP_PORT`, `PAIREE_SFTP_USER`
and `PAIREE_SFTP_PASSWORD` point it at another server.

## Developer Guidelines

Our code architecture values modularity and clean decoupling. All modifications must comply with the guidelines defined in [.agents/AGENTS.md](.agents/AGENTS.md):

- **Single Responsibility Principle (SRP)**: Each source file must do one, well-defined task. No monolithic "god files" are allowed.
- **Zero Hardcoding**: Do not hardcode strings, key names, default paths, or styles. Use translations in `lang/en.toml` or `lang/es.toml`, settings in TOML, and themes.
- **Decoupled State**: UI rendering components (`ratatui`) must be decoupled from the core filesystem layer and event loop.
- **Strict dead-code policy**: Do not use `#[allow(dead_code)]` or `#[allow(unused)]` to bypass compiler warnings. Implement or delete unused code.

## Submitting Pull Requests

- Keep PRs small and focused on a single change.
- Follow the template in `.github/pull_request_template.md`.
- Include visual proof (screenshots or recordings) if you change the UI.
- Update `docs/UNRELEASED.md` in the `[Unreleased]` section with a brief description of user-facing changes.

## Releasing (maintainers)

```sh
cargo xtask release            # or `cargo xtask dev release`
```

The task updates every dependency to its latest compatible version, runs the
local gate (lint, clippy, tests), bumps the version (Cargo.toml and the
WinGet manifests), moves `docs/UNRELEASED.md` into `docs/CHANGELOG.md`, then
commits, tags and pushes the branch and the tag in one atomic push.
`cargo xtask help` lists its options.

It refuses to start with uncommitted changes outside the files it rewrites,
or when the tag already exists locally or on origin. If the gate fails, an
error occurs or you cancel, the release files are put back exactly as they
were. When releasing from a feature branch it merges it into the main branch
only after a fast-forward pull of that branch; otherwise it stops.

The tag starts the Release workflow: it re-runs every check, builds and
smoke-tests all targets, and creates a signed **draft** release. Approve the
`release` environment in the Actions run to publish it; WinGet is submitted
right after.
