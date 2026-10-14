//! Pairee maintenance tasks, run as `cargo xtask <command>` (alias in
//! `.cargo/config.toml`). One cross-platform implementation replaces the
//! former PowerShell and Bash copies of the release and developer scripts.

mod assets;
mod changelog;
mod cmd;
mod dev;
mod lint;
mod registry;
mod release;
// The release asset name table, shared with the updater.
#[path = "../../src/update/assets.rs"]
mod release_assets;
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
      commit, tag and push branch and tag in one atomic push. The tag starts
      the Release workflow. Needs a tree clean apart from the release files;
      a failure or a cancel restores those files as they were.
  changelog <vX.Y.Z|Unreleased>
      Print one changelog section (the GitHub release body).
  lint
      Fast checks run by the pre-push hook: rustfmt, typos, translations and help pages.
  install-hooks
      Point git at .githooks so every `git push` runs `cargo xtask lint`.
  dev [<action>]
      Developer menu (build, run, test, clippy, fmt, clean, WinGet, MSIX,
      release). With an action, runs it directly: `cargo xtask dev list`.
  asset-names <target-triple> <vX.Y.Z>
      Release file names of one build, as KEY=value lines (packaging scripts).
  winget-manifests <vX.Y.Z> <dist-dir> <out-dir>
      WinGet manifests of a release: the versioned folder under manifests/
      with the installer URLs and the SHA-256 from <dist-dir>/*.exe.sha256.
  sign-registry <registry-dir> [--key <file>] [--serial N] [--public-key <key>]
      Sign the plugin registry with minisign: index.toml and blocklist.toml
      with a serial and a timestamp (anti-rollback), every manifest.toml by
      path. Secret key from --key or MINISIGN_SECRET_KEY, like the release.
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
        Some("dev") => dev::run(rest),
        Some("asset-names") => assets::print(rest),
        Some("winget-manifests") => release::winget_manifests(rest),
        Some("sign-registry") => registry::run(rest),
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
