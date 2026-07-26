use anyhow::Result;
use std::env;
use std::process::Command;

/// Checks if the `--standalone` argument was provided.
/// If it is present, it launches a new independent terminal window running
/// the same executable (without the `--standalone` flag) and returns `true`.
/// If the flag is not present, returns `false`.
pub fn check_and_launch_standalone() -> Result<bool> {
    let args: Vec<String> = env::args().collect();

    if let Some(pos) = args.iter().position(|x| x == "--standalone") {
        let current_exe = env::current_exe()?;

        // Strip "--standalone" AND the argv[0] (the binary path that
        // every `Command::arg` invocation is going to ignore anyway).
        // Note: we only strip argv[0] *after* removing --standalone, so
        // even in the edge case where a user renames the binary to
        // "--standalone" (and so argv[0] == "--standalone") we still
        // produce a sensible argv for the child.
        let mut new_args: Vec<String> = args
            .iter()
            .enumerate()
            .filter(|(i, _x)| *i != pos && *i != 0)
            .map(|(_, x)| x.clone())
            .collect();

        #[cfg(target_os = "windows")]
        {
            let mut cmd = Command::new("cmd.exe");
            cmd.arg("/c").arg("start").arg("Pairee").arg(&current_exe);
            new_args.insert(0, current_exe.to_string_lossy().into_owned());
            cmd.args(&new_args);
            cmd.spawn()?;
        }

        #[cfg(target_os = "linux")]
        {
            // The terminal list is now split by display protocol. On
            // Wayland we put the Wayland-native terminals first
            // (`foot`, `wezterm`, `kitty`, `alacritty` — the latter two
            // work in both, but they tend to be installed on Wayland
            // setups) and only fall back to X11 terminals if none of
            // those launched. Without this split, the code would try
            // `gnome-terminal` / `konsole` / `xterm` first on a pure-
            // Wayland Fedora 40+ or Ubuntu 24+ session, fail to even
            // spawn them (no $DISPLAY), and then try `x-terminal-
            // emulator` (the Debian alternatives system) which usually
            // resolves to one of the same X11-only terminals — leaving
            // the user with a "Standalone didn't work" UX failure on
            // a system where the right tool was *just not on the
            // list*.
            let (wayland_terms, x11_terms): (&[&str], &[&str]) = if is_wayland() {
                (
                    &["foot", "wezterm", "kitty", "alacritty"],
                    &[
                        "x-terminal-emulator",
                        "gnome-terminal",
                        "konsole",
                        "xfce4-terminal",
                        "xterm",
                    ],
                )
            } else {
                (
                    &["alacritty", "kitty"],
                    &[
                        "x-terminal-emulator",
                        "gnome-terminal",
                        "konsole",
                        "xfce4-terminal",
                        "foot",
                        "wezterm",
                        "xterm",
                    ],
                )
            };

            let mut spawned = false;
            let mut last_err: Option<std::io::Error> = None;
            for term in wayland_terms.iter().chain(x11_terms.iter()) {
                let mut cmd = Command::new(term);
                cmd.arg("-e").arg(&current_exe);
                cmd.args(&new_args);
                match cmd.spawn() {
                    Ok(_) => {
                        spawned = true;
                        break;
                    }
                    Err(e) => {
                        // Don't bail on the first failure: `gnome-terminal`
                        // might not be installed while `xterm` is. Keep
                        // the last error so the caller can report it
                        // when every candidate has been tried.
                        last_err = Some(e);
                    }
                }
            }

            if !spawned {
                if let Some(e) = last_err {
                    log::warn!(
                        "could not launch any terminal emulator for --standalone (last error: {})",
                        e
                    );
                }
                return Ok(false); // Fallback to running in current terminal
            }
        }

        #[cfg(target_os = "macos")]
        {
            let mut cmd = Command::new("open");
            cmd.arg("-a").arg("Terminal").arg(&current_exe);
            for arg in &new_args {
                cmd.arg("--args").arg(arg);
            }
            cmd.spawn()?;
        }

        return Ok(true);
    }

    Ok(false)
}

/// Returns true if the current process is running under a Wayland
/// compositor. We check both the canonical `WAYLAND_DISPLAY` and
/// `XDG_SESSION_TYPE` (the latter is the more reliable signal on
/// modern distros that may not export `WAYLAND_DISPLAY` even when the
/// session is Wayland-based, e.g. some GNOME-on-XWayland setups).
#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    if env::var_os("WAYLAND_DISPLAY").is_some() {
        return true;
    }
    matches!(env::var("XDG_SESSION_TYPE").as_deref(), Ok("wayland"))
}
