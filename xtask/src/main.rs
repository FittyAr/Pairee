//! Pairee maintenance tasks, run as `cargo xtask <command>` (alias in
//! `.cargo/config.toml`). One cross-platform implementation replaces the
//! former PowerShell and Bash copies of the release scripts.

mod changelog;
mod cmd;
mod lint;
mod release;
mod text;
mod ui;

use std::path::Path;
use std::process::ExitCode;

pub(crate) type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const USAGE: &str = "\
Usage: cargo xtask <command>

Commands:
  release [--version X.Y.Z] [--yes] [--no-update] [--skip-checks] [--no-push]
      Update dependencies to their latest compatible versions, run the local
      gate (lint, clippy, tests), bump the version, stamp the changelog,
      commit, tag and push. The tag starts the Release workflow.
  changelog <vX.Y.Z|Unreleased>
      Print one changelog section (the GitHub release body).
  lint
      Fast checks run by the pre-push hook: rustfmt, typos, translation parity.
  install-hooks
      Point git at .githooks so every `git push` runs `cargo xtask lint`.
";

fn main() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in a subdirectory of the workspace");
    if let Err(e) = std::env::set_current_dir(root) {
        eprintln!("error: cannot enter {}: {e}", root.display());
        return ExitCode::FAILURE;
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest = args.get(1..).unwrap_or_default();
    let result = match args.first().map(String::as_str) {
        Some("release") => release::run(rest),
        Some("changelog") => changelog::print(rest),
        Some("lint") => lint::run(),
        Some("install-hooks") => lint::install_hooks(),
        Some("help" | "-h" | "--help") | None => {
            print!("{USAGE}");
            Ok(())
        }
        Some(other) => Err(format!("unknown command '{other}'\n\n{USAGE}").into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            ui::error(&e.to_string());
            ExitCode::FAILURE
        }
    }
}
