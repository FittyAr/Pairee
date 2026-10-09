//! Running external programs (git, cargo, gh, typos).

use std::process::{Command, Stdio};

use crate::Result;

fn describe(program: &str, args: &[&str]) -> String {
    format!("`{program} {}`", args.join(" "))
}

/// Runs with inherited output; fails on a non-zero exit.
pub fn run(program: &str, args: &[&str]) -> Result {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| format!("cannot start {program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} failed ({status})", describe(program, args)).into())
    }
}

/// Returns the trimmed standard output; fails on a non-zero exit.
pub fn output(program: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(program)
        .args(args)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("cannot start {program}: {e}"))?;
    if !out.status.success() {
        return Err(format!("{} failed ({})", describe(program, args), out.status).into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Runs `command` with its output discarded; true when it exits successfully.
pub fn quietly(command: &mut Command) -> bool {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Whether `program --version` can be started and succeeds.
pub fn available(program: &str) -> bool {
    quietly(Command::new(program).arg("--version"))
}
