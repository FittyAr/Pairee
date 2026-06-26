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

Our custom build script `build.rs` automatically captures target compilation, git commit hash, and profile info so you don't get "unknown" values in the About dialog.

### Formatting & Lints

Before submitting code, you must ensure that formatting and lints pass:

1. **Format Code**: Check and write formatting.
   ```sh
   cargo fmt --all -- --check
   ```
2. **Clippy Lints**: Run clippy check.
   ```sh
   cargo clippy --all-targets -- -D warnings
   ```
3. **Unit Tests**: Run tests.
   ```sh
   cargo test
   ```

## Developer Guidelines

Our code architecture values modularity and clean decoupling. All modifications must comply with the guidelines defined in [.agents/AGENTS.md](file:///.agents/AGENTS.md):

- **Single Responsibility Principle (SRP)**: Each source file must do one, well-defined task. No monolithic "god files" are allowed.
- **Zero Hardcoding**: Do not hardcode strings, key names, default paths, or styles. Use translations in `src/config/localization/en.rs` or `lang/es.json`, settings in TOML, and themes.
- **Decoupled State**: UI rendering components (`ratatui`) must be decoupled from the core filesystem layer and event loop.
- **Strict dead-code policy**: Do not use `#[allow(dead_code)]` or `#[allow(unused)]` to bypass compiler warnings. Implement or delete unused code.

## Submitting Pull Requests

- Keep PRs small and focused on a single change.
- Follow the template in `.github/pull_request_template.md`.
- Include visual proof (screenshots or recordings) if you change the UI.
- Update `CHANGELOG.md` in the `[Unreleased]` section with a brief description of user-facing changes.
