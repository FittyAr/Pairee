//! Fast local checks (the same ones CI runs first) and the git hook that runs
//! them before every push.

use crate::{Result, cmd, ui};

/// rustfmt, typos and EN/ES translation parity. Tools that are not installed
/// are reported and skipped; CI still runs them.
pub fn run() -> Result {
    ui::step("rustfmt");
    cmd::run("cargo", &["fmt", "--all", "--", "--check"])
        .map_err(|_| "code is not formatted: run `cargo fmt --all`")?;

    if cmd::available("typos") {
        ui::step("typos");
        cmd::run("typos", &[])?;
    } else {
        ui::warn("typos is not installed (`cargo install typos-cli`); skipping the spell check");
    }

    match ["python3", "python"]
        .into_iter()
        .find(|p| cmd::available(p))
    {
        Some(python) => {
            ui::step("translation key parity");
            cmd::run(python, &["scripts/check_translations.py"])?;
        }
        None => ui::warn("Python is not installed; skipping the translation key check"),
    }
    ui::ok("lint passed");
    Ok(())
}

/// Points git at the versioned hooks in `.githooks/`.
pub fn install_hooks() -> Result {
    cmd::run("git", &["config", "core.hooksPath", ".githooks"])?;
    ui::ok(
        "git hooks installed: `git push` now runs `cargo xtask lint` (skip once with --no-verify)",
    );
    Ok(())
}
