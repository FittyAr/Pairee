use anyhow::Result;
use std::env;
use std::ffi::OsString;
#[cfg(not(target_os = "windows"))]
use std::process::Command;

use crate::launch_args::STANDALONE;

/// Launches a new independent terminal window running the same executable
/// with `args` (the full command line, program first, already validated)
/// minus the `--standalone` option, and returns `true`; `false` when
/// `args` has no such option (before `--`).
pub fn check_and_launch_standalone(args: &[OsString]) -> Result<bool> {
    let Some(pos) = args
        .iter()
        .take_while(|a| *a != "--")
        .position(|x| x == STANDALONE)
    else {
        return Ok(false);
    };
    let current_exe = env::current_exe()?;
    let new_args: Vec<&OsString> = args
        .iter()
        .enumerate()
        .skip(1) // the executable path itself
        .filter(|(i, _)| *i != pos)
        .map(|(_, a)| a)
        .collect();

    #[cfg(target_os = "windows")]
    {
        // `start`'s first quoted argument is the window title; every other
        // argument is quoted for cmd (a folder like `R&D` stays whole).
        let line = shell_line(&current_exe, &new_args);
        crate::shell::shell_command(&format!("start \"Pairee\" {line}")).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        let terminals = [
            "x-terminal-emulator",
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "alacritty",
            "kitty",
            "xterm",
        ];

        let mut spawned = false;
        for term in terminals {
            let mut cmd = Command::new(term);
            // gnome-terminal's `-e` takes one string; `--` runs the argv.
            cmd.arg(if term == "gnome-terminal" { "--" } else { "-e" })
                .arg(&current_exe)
                .args(&new_args);

            if cmd.spawn().is_ok() {
                spawned = true;
                break;
            }
        }

        if !spawned {
            return Ok(false); // Fallback to running in current terminal
        }
    }

    #[cfg(target_os = "macos")]
    {
        // `open --args` would hand the arguments to Terminal, not to
        // Pairee: Terminal runs a shell command line instead.
        let line = shell_line(&current_exe, &new_args);
        let applescript_string = line.replace('\\', "\\\\").replace('"', "\\\"");
        Command::new("osascript")
            .arg("-e")
            .arg(format!(
                "tell application \"Terminal\" to do script \"{applescript_string}\""
            ))
            .spawn()?;
    }

    Ok(true)
}

/// `exe` and `args` as one command line for the platform shell.
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn shell_line(exe: &std::path::Path, args: &[&OsString]) -> String {
    std::iter::once(exe.as_os_str())
        .chain(args.iter().map(|a| a.as_os_str()))
        .map(|a| crate::shell::quote_native(&a.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ")
}
