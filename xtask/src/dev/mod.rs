//! `cargo xtask dev`: the developer menu (formerly run.bat and run.sh), and
//! `cargo xtask dev <action>` to run one entry directly.

mod msix;

use std::process::Command;

use crate::{Result, cmd, release, ui};

/// What an entry runs.
enum Task {
    /// `cargo <args>`.
    Cargo(&'static [&'static str]),
    /// `winget <args>` (Windows).
    Winget(&'static [&'static str]),
    /// An xtask function.
    Call(fn() -> Result),
}

struct Action {
    name: &'static str,
    label: &'static str,
    task: Task,
}

const WINGET_ID: &str = "FittyAr.Pairee";

static ACTIONS: &[Action] = &[
    Action {
        name: "run",
        label: "Run Pairee (cargo run)",
        task: Task::Cargo(&["run"]),
    },
    Action {
        name: "standalone",
        label: "Run Pairee standalone (cargo run -- --standalone)",
        task: Task::Cargo(&["run", "--", "--standalone"]),
    },
    Action {
        name: "build",
        label: "Debug build (cargo build)",
        task: Task::Cargo(&["build"]),
    },
    Action {
        name: "build-release",
        label: "Release build (cargo build --release)",
        task: Task::Cargo(&["build", "--release"]),
    },
    Action {
        name: "test",
        label: "Tests (cargo test --workspace)",
        task: Task::Cargo(&["test", "--workspace"]),
    },
    Action {
        name: "check",
        label: "Compiler check (cargo check --workspace --all-targets)",
        task: Task::Cargo(&["check", "--workspace", "--all-targets"]),
    },
    Action {
        name: "clippy",
        label: "Clippy (cargo clippy --workspace --all-targets -- -D warnings)",
        task: Task::Cargo(&[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]),
    },
    Action {
        name: "fmt",
        label: "Format check (cargo fmt --all -- --check)",
        task: Task::Cargo(&["fmt", "--all", "--", "--check"]),
    },
    Action {
        name: "lint",
        label: "Pre-push checks (cargo xtask lint)",
        task: Task::Call(crate::lint::run),
    },
    Action {
        name: "clean",
        label: "Clean the build directory (cargo clean)",
        task: Task::Call(clean),
    },
    Action {
        name: "winget-install",
        label: "WinGet: install Pairee (architecture auto-detected)",
        task: Task::Winget(&[
            "install",
            WINGET_ID,
            "--accept-source-agreements",
            "--accept-package-agreements",
        ]),
    },
    Action {
        name: "winget-install-x64",
        label: "WinGet: install Pairee (x64)",
        task: Task::Winget(&[
            "install",
            WINGET_ID,
            "--architecture",
            "x64",
            "--accept-source-agreements",
            "--accept-package-agreements",
        ]),
    },
    Action {
        name: "winget-install-arm64",
        label: "WinGet: install Pairee (ARM64)",
        task: Task::Winget(&[
            "install",
            WINGET_ID,
            "--architecture",
            "arm64",
            "--accept-source-agreements",
            "--accept-package-agreements",
        ]),
    },
    Action {
        name: "winget-upgrade",
        label: "WinGet: upgrade Pairee",
        task: Task::Winget(&[
            "upgrade",
            WINGET_ID,
            "--accept-source-agreements",
            "--accept-package-agreements",
        ]),
    },
    Action {
        name: "winget-uninstall",
        label: "WinGet: uninstall Pairee",
        task: Task::Winget(&["uninstall", WINGET_ID]),
    },
    Action {
        name: "msix-package",
        label: "MSIX: package the release build (target/pairee_local_<arch>.msix)",
        task: Task::Call(msix::package),
    },
    Action {
        name: "msix-cert",
        label: "MSIX: create and trust a self-signed test certificate (elevated shell)",
        task: Task::Call(msix::create_certificate),
    },
    Action {
        name: "msix-sign",
        label: "MSIX: sign the local package with the test certificate",
        task: Task::Call(msix::sign),
    },
    Action {
        name: "msix-install",
        label: "MSIX: install the signed local package",
        task: Task::Call(msix::install),
    },
    Action {
        name: "release",
        label: "Bump the version and release (cargo xtask release)",
        task: Task::Call(release_interactively),
    },
];

fn release_interactively() -> Result {
    release::run(&[])
}

/// `cargo clean`. Windows cannot delete the running xtask.exe, so there the
/// clean starts once this process has exited (and the menu closes).
fn clean() -> Result {
    if !cfg!(windows) {
        return cmd::run("cargo", &["clean"]);
    }
    Command::new("cmd")
        .args(["/C", "ping -n 3 127.0.0.1 >NUL & cargo clean"])
        .spawn()
        .map_err(|e| format!("cannot start cargo clean: {e}"))?;
    ui::ok("`cargo clean` runs as soon as xtask has exited");
    std::process::exit(0);
}

/// `cargo xtask dev [<action>|list]`.
pub fn run(args: &[String]) -> Result {
    match args {
        [] => menu(),
        [name] if name == "list" => {
            for action in ACTIONS {
                println!("  {:<22}{}", action.name, action.label);
            }
            Ok(())
        }
        [name] => perform(find(name)?),
        _ => Err("usage: cargo xtask dev [<action>|list]".into()),
    }
}

fn find(name: &str) -> Result<&'static Action> {
    ACTIONS
        .iter()
        .find(|a| a.name == name)
        .ok_or_else(|| format!("unknown action '{name}' (see `cargo xtask dev list`)").into())
}

fn perform(action: &Action) -> Result {
    ui::step(action.label);
    match &action.task {
        Task::Cargo(args) => cmd::run("cargo", args),
        Task::Winget(args) => {
            if !cfg!(windows) {
                return Err("WinGet is only available on Windows".into());
            }
            cmd::run("winget", args)
        }
        Task::Call(f) => f(),
    }
}

/// Numbered menu; an entry can be picked by number or by name.
fn menu() -> Result {
    loop {
        println!();
        println!("Pairee developer menu");
        for (i, action) in ACTIONS.iter().enumerate() {
            println!("  {:>2}. {:<22}{}", i + 1, action.name, action.label);
        }
        println!("   q. quit");
        let choice = ui::ask("Choose an action", "q")?;
        if choice.eq_ignore_ascii_case("q") {
            return Ok(());
        }
        let action = match choice.parse::<usize>() {
            Ok(n) => ACTIONS.get(n.wrapping_sub(1)).ok_or("no such entry"),
            Err(_) => find(&choice).map_err(|_| "no such entry"),
        };
        match action {
            Ok(action) => match perform(action) {
                Ok(()) => ui::ok("done"),
                Err(e) => ui::error(&e.to_string()),
            },
            Err(e) => ui::warn(e),
        }
        ui::pause()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_names_are_unique_and_found() {
        for (i, action) in ACTIONS.iter().enumerate() {
            assert!(ACTIONS[i + 1..].iter().all(|b| b.name != action.name));
            assert!(std::ptr::eq(find(action.name).unwrap(), action));
        }
        assert!(find("nope").is_err());
    }
}
