//! `cargo xtask release`: bump the version, commit, tag and push. The pushed
//! tag starts `.github/workflows/release.yml`, which re-runs the Check
//! workflow before building anything.
//!
//! Until 1.0 every release ships with the latest compatible version of every
//! dependency: the lockfile is refreshed with `cargo update` and the full local
//! gate runs on the result before anything is committed.

use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::text::TextFile;
use crate::{Result, changelog, cmd, lint, ui};

const WINGET_DIR: &str = "manifests/f/FittyAr/Pairee";
const RELEASE_WORKFLOW: &str = "Release";
const CHECK_WORKFLOW: &str = "Check";

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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Version(u64, u64, u64);

impl Version {
    fn next_patch(self) -> Self {
        Self(self.0, self.1, self.2 + 1)
    }
}

impl std::str::FromStr for Version {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, String> {
        let invalid = || format!("invalid version '{s}' (expected X.Y.Z)");
        let mut parts = s.trim().trim_start_matches('v').split('.');
        let mut next = || -> std::result::Result<u64, String> {
            parts
                .next()
                .ok_or_else(invalid)?
                .parse()
                .map_err(|_| invalid())
        };
        let version = Self(next()?, next()?, next()?);
        match parts.next() {
            None => Ok(version),
            Some(_) => Err(invalid()),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

pub fn run(args: &[String]) -> Result {
    let opts = Options::parse(args)?;
    ui::step("Pairee version bump & release");

    let branch = prepare_branch(opts.yes)?;
    if !opts.no_push {
        ensure_push_access(&branch)?;
    }
    if !git_status()?.is_empty() {
        ui::warn(&format!("uncommitted changes:\n{}", git_status()?));
        if !ui::confirm(
            "Release anyway (only the release files are committed)?",
            opts.yes,
        )? {
            return Err("aborted".into());
        }
    }
    if !opts.skip_checks {
        check_remote_ci(&branch)?;
    }

    let current = cargo_version()?;
    let new = choose_version(current, opts.version)?;

    if !opts.no_update {
        update_dependencies()?;
    }
    if !opts.skip_checks
        && let Err(e) = local_gate()
    {
        if !opts.no_update {
            ui::warn("restoring Cargo.lock (the dependency update stays out of the release)");
            cmd::run("git", &["checkout", "--", "Cargo.lock"])?;
        }
        return Err(format!("local gate failed, nothing was bumped: {e}").into());
    }

    if !changelog::has_unreleased_notes()?
        && !ui::confirm("docs/UNRELEASED.md has no notes. Release anyway?", opts.yes)?
    {
        return Err("aborted".into());
    }

    let files = bump_files(current, new)?;
    ui::step("Refreshing Cargo.lock for the new version");
    cmd::run("cargo", &["update", "--workspace"])?;

    let tag = format!("v{new}");
    println!();
    println!("Ready to release {tag} from '{branch}':");
    println!("  - commit {}", files.join(", "));
    println!("  - create tag {tag}");
    if !opts.no_push {
        println!("  - push '{branch}' and {tag} to origin (starts the Release workflow)");
    }
    if !ui::confirm("Proceed?", opts.yes)? {
        ui::warn(
            "cancelled: the files are bumped but nothing was committed (`git checkout -- .` reverts them)",
        );
        return Ok(());
    }

    let mut add = vec!["add", "--all", "--"];
    add.extend(files.iter().map(String::as_str));
    cmd::run("git", &add)?;
    cmd::run("git", &["commit", "-m", &format!("Bump version to {tag}")])?;
    cmd::run("git", &["tag", "-a", &tag, "-m", &format!("Release {tag}")])?;

    if opts.no_push {
        ui::ok(&format!(
            "{tag} committed and tagged locally; push with: git push origin {branch} {tag}"
        ));
        return Ok(());
    }
    cmd::run("git", &["push", "origin", &branch])?;
    cmd::run("git", &["push", "origin", &tag])?;
    ui::ok(&format!("pushed {tag}"));
    ensure_release_started(&tag);
    Ok(())
}

// ── Branch and repository state ─────────────────────────────────────────────

fn git_status() -> Result<String> {
    cmd::output("git", &["status", "--porcelain"])
}

fn branch_exists(name: &str) -> bool {
    cmd::quietly(Command::new("git").args([
        "show-ref",
        "--verify",
        "--quiet",
        &format!("refs/heads/{name}"),
    ]))
}

/// Releases are cut from the main branch: offers to merge a feature branch
/// into it first. Returns the branch to release from.
fn prepare_branch(yes: bool) -> Result<String> {
    let main = if branch_exists("master") && !branch_exists("main") {
        "master"
    } else {
        "main"
    };
    let branch = cmd::output("git", &["branch", "--show-current"])?;
    if branch.is_empty() {
        return Err("HEAD is detached; check out a branch first".into());
    }
    if branch == main {
        return Ok(branch);
    }

    ui::warn(&format!(
        "you are on '{branch}', not on the release branch '{main}'"
    ));
    if ui::confirm(
        &format!("Merge '{branch}' into '{main}' and release from '{main}'?"),
        yes,
    )? {
        if !git_status()?.is_empty() {
            return Err(format!("commit or stash the changes in '{branch}' first").into());
        }
        cmd::run("git", &["checkout", main])?;
        if cmd::run("git", &["pull", "--ff-only", "origin", main]).is_err() {
            ui::warn(&format!(
                "could not fast-forward '{main}' from origin; merging anyway"
            ));
        }
        cmd::run("git", &["merge", &branch]).map_err(|_| {
            format!("merging '{branch}' into '{main}' failed: resolve the conflicts and re-run")
        })?;
        return Ok(main.to_string());
    }
    if ui::confirm(&format!("Release directly from '{branch}'?"), false)? {
        Ok(branch)
    } else {
        Err("aborted".into())
    }
}

fn ensure_push_access(branch: &str) -> Result {
    ui::step(&format!("Checking push access to origin/{branch}"));
    let ok = cmd::quietly(
        Command::new("git")
            .args(["push", "--dry-run", "origin", branch])
            .env("GIT_TERMINAL_PROMPT", "0"),
    );
    if ok {
        Ok(())
    } else {
        Err(format!(
            "cannot push to origin (log in with `gh auth login` or your credential helper; \
             test with `git push --dry-run origin {branch}`)"
        )
        .into())
    }
}

/// `gh` is optional; without it (or unauthenticated) GitHub is not queried.
fn gh_ready() -> bool {
    cmd::available("gh") && cmd::quietly(Command::new("gh").args(["auth", "status"]))
}

/// Refuses to release a commit whose Check run failed on GitHub (the Release
/// workflow would stop at the same failure after the tag is out).
fn check_remote_ci(branch: &str) -> Result {
    if !gh_ready() {
        ui::warn("gh is not available or not logged in; skipping the GitHub Check status");
        return Ok(());
    }
    let head = cmd::output("git", &["rev-parse", "HEAD"])?;
    let runs = cmd::output(
        "gh",
        &[
            "run",
            "list",
            "--workflow",
            CHECK_WORKFLOW,
            "--branch",
            branch,
            "--limit",
            "20",
            "--json",
            "headSha,status,conclusion,url",
            "--jq",
            r#".[] | "\(.headSha) \(.status) \(.conclusion) \(.url)""#,
        ],
    )?;
    let Some(run) = runs.lines().find(|l| l.starts_with(&head)) else {
        ui::warn(
            "HEAD has no Check run on GitHub yet; the Release workflow runs it before building",
        );
        return Ok(());
    };
    let fields: Vec<&str> = run.split(' ').collect();
    match (fields.get(1), fields.get(2)) {
        (Some(&"completed"), Some(&"success")) => {
            ui::ok("GitHub Check passed for HEAD");
            Ok(())
        }
        (Some(&"completed"), Some(conclusion)) => Err(format!(
            "the Check workflow ended '{conclusion}' for HEAD: {}\nFix it first (or pass --skip-checks)",
            fields.get(3).unwrap_or(&"")
        )
        .into()),
        _ => {
            ui::warn("the Check workflow is still running for HEAD; the Release workflow waits for it again");
            Ok(())
        }
    }
}

// ── Version and dependencies ────────────────────────────────────────────────

/// `version` in the `[package]` table of Cargo.toml.
fn cargo_version() -> Result<Version> {
    let manifest = TextFile::load("Cargo.toml")?;
    let line = package_version_line(&manifest.lines).ok_or("no [package] version in Cargo.toml")?;
    let value = manifest.lines[line]
        .split('"')
        .nth(1)
        .ok_or("malformed version line")?;
    Ok(value.parse()?)
}

fn package_version_line(lines: &[String]) -> Option<usize> {
    let mut in_package = false;
    lines.iter().position(|l| {
        let l = l.trim();
        if l.starts_with('[') {
            in_package = l == "[package]";
        }
        in_package && l.starts_with("version") && l[7..].trim_start().starts_with('=')
    })
}

fn choose_version(current: Version, requested: Option<Version>) -> Result<Version> {
    println!("Current version: {current}");
    let new = match requested {
        Some(v) => v,
        None => ui::ask("New version", &current.next_patch().to_string())?.parse()?,
    };
    if new <= current {
        return Err(format!("{new} is not newer than {current}").into());
    }
    if !cmd::output("git", &["tag", "--list", &format!("v{new}")])?.is_empty() {
        return Err(format!("tag v{new} already exists").into());
    }
    Ok(new)
}

/// `cargo update` moves every dependency to the newest version allowed by
/// Cargo.toml and `rust-version`. `--verbose` lists the ones held back (a new
/// major version, or a newer MSRV): raising those is a manual Cargo.toml edit.
fn update_dependencies() -> Result {
    ui::step("Updating dependencies to their latest compatible versions");
    cmd::run("cargo", &["update", "--verbose"])?;
    let changed = cmd::output("git", &["diff", "--stat", "--", "Cargo.lock"])?;
    if changed.is_empty() {
        ui::ok("all dependencies were already up to date");
    }
    Ok(())
}

/// The CI checks that can run locally: lint, clippy and the test suite.
fn local_gate() -> Result {
    ui::step("Local gate: lint, clippy, tests");
    lint::run()?;
    ui::step("clippy");
    cmd::run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    ui::step("tests");
    cmd::run("cargo", &["test", "--workspace", "--locked"])?;
    ui::ok("local gate passed");
    Ok(())
}

// ── Version stamps ──────────────────────────────────────────────────────────

/// Writes `new` everywhere the version appears; returns the paths to commit.
fn bump_files(current: Version, new: Version) -> Result<Vec<String>> {
    ui::step(&format!("Bumping {current} -> {new}"));
    let mut files = vec!["Cargo.toml".to_string(), "Cargo.lock".to_string()];

    let mut manifest = TextFile::load("Cargo.toml")?;
    let line = package_version_line(&manifest.lines).ok_or("no [package] version in Cargo.toml")?;
    manifest.lines[line] = format!("version = \"{new}\"");
    manifest.save()?;

    let mut installer = TextFile::load("installer.iss")?;
    if installer.replace_line(
        |l| l.starts_with("#define AppVersion"),
        |_| format!("#define AppVersion \"{new}\""),
    ) {
        installer.save()?;
        files.push("installer.iss".into());
    } else {
        ui::warn("installer.iss has no `#define AppVersion` line");
    }

    if let Some(dir) = bump_winget_manifests(current, new)? {
        files.push(dir);
    }

    changelog::stamp(&new.to_string())?;
    files.extend(["docs/CHANGELOG.md".into(), "docs/UNRELEASED.md".into()]);
    Ok(files)
}

/// Moves the WinGet manifests to the new version folder and updates the
/// version, installer URLs and release-notes URL in them.
fn bump_winget_manifests(current: Version, new: Version) -> Result<Option<String>> {
    let base = Path::new(WINGET_DIR);
    let from = base.join(current.to_string());
    let from = if from.is_dir() {
        from
    } else {
        match fs::read_dir(base)
            .ok()
            .and_then(|mut d| d.find_map(|e| e.ok()))
        {
            Some(entry) => entry.path(),
            None => {
                ui::warn("no WinGet manifest folder found; skipping");
                return Ok(None);
            }
        }
    };
    let to = base.join(new.to_string());
    if from != to {
        fs::rename(&from, &to)?;
    }
    for entry in fs::read_dir(&to)? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "yaml") {
            continue;
        }
        let mut manifest = TextFile::load(&path)?;
        for line in &mut manifest.lines {
            let key = line.trim_start();
            if key.starts_with("PackageVersion:") {
                *line = format!("PackageVersion: {new}");
            } else if key.starts_with("InstallerUrl:") || key.starts_with("ReleaseNotesUrl:") {
                *line = replace_versions(line, new);
            }
        }
        manifest.save()?;
    }
    Ok(Some(WINGET_DIR.to_string()))
}

/// Replaces every `X.Y.Z` in `line` with `new` (a leading `v` is kept).
fn replace_versions(line: &str, new: Version) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find(|c: char| c.is_ascii_digit()) {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let len = tail
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(tail.len());
        let token = tail[..len].trim_end_matches('.');
        if token.parse::<Version>().is_ok() {
            out.push_str(&new.to_string());
        } else {
            out.push_str(token);
        }
        rest = &tail[token.len()..];
    }
    out.push_str(rest);
    out
}

// ── After the push ──────────────────────────────────────────────────────────

/// GitHub occasionally drops the push event of a tag; when no Release run
/// shows up within a minute, starts one by hand.
fn ensure_release_started(tag: &str) {
    if !gh_ready() {
        ui::warn("gh is not available; check the Actions tab for the Release run");
        return;
    }
    ui::step("Waiting for the Release workflow to start");
    let query = format!(r#".[] | select(.event == "push" and .headBranch == "{tag}") | .url"#);
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        sleep(Duration::from_secs(5));
        let found = cmd::output(
            "gh",
            &[
                "run",
                "list",
                "--workflow",
                RELEASE_WORKFLOW,
                "--limit",
                "10",
                "--json",
                "event,headBranch,url",
                "--jq",
                &query,
            ],
        );
        if let Ok(url) = found
            && !url.is_empty()
        {
            ui::ok(&format!(
                "Release workflow running: {}",
                url.lines().next().unwrap_or("")
            ));
            println!(
                "When it finishes, approve the 'release' environment to publish the draft release."
            );
            return;
        }
    }
    ui::warn("no Release run appeared; starting it manually");
    if cmd::run("gh", &["workflow", "run", RELEASE_WORKFLOW, "--ref", tag]).is_err() {
        ui::error(&format!(
            "start it with: gh workflow run {RELEASE_WORKFLOW} --ref {tag}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_parse_and_order() {
        assert_eq!("v0.9.0".parse::<Version>(), Ok(Version(0, 9, 0)));
        assert!("0.9".parse::<Version>().is_err());
        assert!("0.9.0.1".parse::<Version>().is_err());
        assert!(Version(0, 10, 0) > Version(0, 9, 9));
        assert_eq!(Version(0, 9, 0).next_patch(), Version(0, 9, 1));
    }

    #[test]
    fn urls_get_the_new_version() {
        let new = Version(1, 2, 3);
        assert_eq!(
            replace_versions(
                "    InstallerUrl: https://github.com/FittyAr/Pairee/releases/download/v0.9.0/pairee-setup-0.9.0-x64.exe",
                new
            ),
            "    InstallerUrl: https://github.com/FittyAr/Pairee/releases/download/v1.2.3/pairee-setup-1.2.3-x64.exe"
        );
        assert_eq!(
            replace_versions("schema.1.12.0.json", new),
            "schema.1.2.3.json"
        );
        assert_eq!(replace_versions("x64 and 2026", new), "x64 and 2026");
    }

    #[test]
    fn package_version_ignores_other_tables() {
        let lines: Vec<String> = [
            "[dependencies]",
            "version = \"1\"",
            "[package]",
            "name = \"p\"",
            "version = \"0.1.0\"",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(package_version_line(&lines), Some(4));
    }
}
