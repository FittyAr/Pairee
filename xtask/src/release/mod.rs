//! `cargo xtask release`: bump the version, commit, tag and push. The pushed
//! tag starts `.github/workflows/release.yml`, which re-runs the Check
//! workflow before building anything.
//!
//! Until 1.0 every release ships with the latest compatible version of every
//! dependency: the lockfile is refreshed with `cargo update` and the full local
//! gate runs on the result before anything is committed.
//!
//! The working tree must be clean apart from the release files. Those are
//! snapshotted first; a failed gate, an error or a cancel puts them back as
//! they were (see [`Snapshot`]).

mod branch;
mod ci;
mod snapshot;
pub mod version;
mod winget;

pub use winget::manifests_for_release as winget_manifests;

use std::path::PathBuf;

use crate::{Result, changelog, cmd, ui};
use snapshot::Snapshot;
use version::Version;

#[derive(Default)]
struct Options {
    version: Option<Version>,
    yes: bool,
    no_update: bool,
    skip_checks: bool,
    no_push: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self> {
        let mut opts = Self::default();
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--version" => {
                    let value = args.next().ok_or("--version needs a value")?;
                    opts.version = Some(value.parse()?);
                }
                "--yes" | "-y" => opts.yes = true,
                "--no-update" => opts.no_update = true,
                "--skip-checks" => opts.skip_checks = true,
                "--no-push" => opts.no_push = true,
                other => return Err(format!("unknown release option '{other}'").into()),
            }
        }
        Ok(opts)
    }
}

pub fn run(args: &[String]) -> Result {
    let opts = Options::parse(args)?;
    ui::step("Pairee version bump & release");

    branch::ensure_clean_tree()?;
    let branch = branch::prepare(opts.yes)?;
    if !opts.no_push {
        branch::ensure_push_access(&branch)?;
    }
    if !opts.skip_checks {
        ci::check_remote_ci(&branch)?;
    }

    let current = version::cargo_version()?;
    let new = version::choose(current, opts.version, !opts.no_push)?;
    let tag = format!("v{new}");

    let mut snapshot = Snapshot::take(release_files(current)?)?;
    let files = match prepare_files(&opts, current, new, &mut snapshot) {
        Ok(Some(files)) => files,
        Ok(None) => {
            snapshot.restore_or_warn();
            return Err("aborted".into());
        }
        Err(e) => {
            snapshot.restore_or_warn();
            return Err(e);
        }
    };

    println!();
    println!("Ready to release {tag} from '{branch}':");
    println!("  - commit {}", files.join(", "));
    println!("  - create tag {tag}");
    if !opts.no_push {
        println!(
            "  - push '{branch}' and {tag} to origin in one atomic push (starts the Release workflow)"
        );
    }
    if !ui::confirm("Proceed?", opts.yes)? {
        snapshot.restore_or_warn();
        ui::warn("cancelled: nothing was committed");
        return Ok(());
    }

    branch::commit_and_tag(&files, &tag)?;
    if opts.no_push {
        ui::ok(&format!(
            "{tag} committed and tagged locally; push with: git push --atomic origin {branch} {tag}"
        ));
        return Ok(());
    }
    branch::push(&branch, &tag)?;
    ui::ok(&format!("pushed {tag}"));
    ci::ensure_release_started(&tag);
    Ok(())
}

/// The files every release rewrites, besides the WinGet manifest folder.
/// (installer.iss gets its version from the packaging script.)
pub(super) const RELEASE_FILES: [&str; 4] = [
    version::CARGO_TOML,
    version::CARGO_LOCK,
    changelog::CHANGELOG,
    changelog::UNRELEASED,
];

/// Every file the release may rewrite, as it is now.
fn release_files(current: Version) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = RELEASE_FILES.map(PathBuf::from).to_vec();
    if let Some(dir) = winget::source_dir(current) {
        files.extend(winget::manifests(&dir)?);
    }
    Ok(files)
}

/// Dependency update, local gate and version bump. `None` when the user
/// declines to release without notes.
fn prepare_files(
    opts: &Options,
    current: Version,
    new: Version,
    snapshot: &mut Snapshot,
) -> Result<Option<Vec<String>>> {
    if !opts.no_update {
        ci::update_dependencies()?;
    }
    if !opts.skip_checks {
        ci::local_gate().map_err(|e| format!("local gate failed, nothing was bumped: {e}"))?;
    }
    if !changelog::has_unreleased_notes()?
        && !ui::confirm("docs/UNRELEASED.md has no notes. Release anyway?", opts.yes)?
    {
        return Ok(None);
    }

    let files = version::bump_files(current, new, snapshot)?;
    ui::step("Refreshing Cargo.lock for the new version");
    cmd::run("cargo", &["update", "--workspace"])?;
    Ok(Some(files))
}
