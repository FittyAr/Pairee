//! Checks around a release: the dependency refresh and local gate before
//! it, the GitHub Check status of HEAD, and the Release run after the push.

use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::{Result, cmd, lint, ui};

const RELEASE_WORKFLOW: &str = "Release";
const CHECK_WORKFLOW: &str = "Check";

/// `cargo update` moves every dependency to the newest version allowed by
/// Cargo.toml and `rust-version`. `--verbose` lists the ones held back (a new
/// major version, or a newer MSRV): raising those is a manual Cargo.toml edit.
pub fn update_dependencies() -> Result {
    ui::step("Updating dependencies to their latest compatible versions");
    cmd::run("cargo", &["update", "--verbose"])?;
    let changed = cmd::output("git", &["diff", "--stat", "--", "Cargo.lock"])?;
    if changed.is_empty() {
        ui::ok("all dependencies were already up to date");
    }
    Ok(())
}

/// The CI checks that can run locally: lint, clippy and the test suite.
pub fn local_gate() -> Result {
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

/// `gh` is optional; without it (or unauthenticated) GitHub is not queried.
fn gh_ready() -> bool {
    cmd::available("gh") && cmd::quietly(Command::new("gh").args(["auth", "status"]))
}

/// Refuses to release a commit whose Check run failed on GitHub (the Release
/// workflow would stop at the same failure after the tag is out).
pub fn check_remote_ci(branch: &str) -> Result {
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

/// GitHub occasionally drops the push event of a tag; when no Release run
/// shows up within a minute, starts one by hand.
pub fn ensure_release_started(tag: &str) {
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
