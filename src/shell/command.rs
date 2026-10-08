//! Running a user command line through the platform shell.
//!
//! Unix: `sh -c <script>`.
//!
//! Windows: `cmd.exe /V:OFF /S /C "<script>"`, passed with `raw_arg` so Rust
//! does not apply MSVCRT escaping (`"` -> `\"`) that cmd does not understand.
//! `/S` makes cmd strip exactly the outer quotes and run the rest verbatim;
//! `/V:OFF` disables delayed `!VAR!` expansion regardless of the registry.

/// Environment variable carrying the script when a raw command line cannot
/// be passed (portable-pty quotes every argument itself).
#[cfg(windows)]
pub const SCRIPT_ENV_VAR: &str = "PAIREE_SHELL_SCRIPT";

#[cfg(windows)]
const CMD_SWITCHES: &str = "/V:OFF /S /C";

#[cfg(windows)]
fn comspec() -> String {
    std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".to_string())
}

/// `std::process::Command` that runs `script` in the platform shell.
pub fn shell_command(script: &str) -> std::process::Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new(comspec());
        cmd.raw_arg(format!("{CMD_SWITCHES} \"{script}\""));
        cmd
    }
    #[cfg(not(windows))]
    {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(script);
        cmd
    }
}

/// Program, argv and extra environment that run `script` in the platform
/// shell when the caller can only pass individually quoted arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellInvocation {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// See [`ShellInvocation`]. On Windows the script travels in
/// `SCRIPT_ENV_VAR` and cmd expands it once (the argument
/// `%PAIREE_SHELL_SCRIPT%` contains no space or quote, so no argument
/// quoting touches it); the user's own `%VAR%` references are pre-expanded
/// with cmd's rules because cmd does not re-expand a variable's value.
pub fn shell_invocation(script: &str) -> ShellInvocation {
    #[cfg(windows)]
    {
        let expanded = super::cmd_env::expand_percent_vars(script, |name| std::env::var(name).ok());
        let mut args: Vec<String> = CMD_SWITCHES.split(' ').map(str::to_string).collect();
        args.push(format!("%{SCRIPT_ENV_VAR}%"));
        ShellInvocation {
            program: comspec(),
            args,
            env: vec![(SCRIPT_ENV_VAR.to_string(), expanded)],
        }
    }
    #[cfg(not(windows))]
    {
        ShellInvocation {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), script.to_string()],
            env: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::quote::quote_native;

    fn run_std(script: &str) -> String {
        let out = shell_command(script)
            .env("PAIREE_QUOTE_TEST", "EXPANDED")
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn run_invocation(script: &str) -> String {
        let inv = shell_invocation(script);
        let mut cmd = std::process::Command::new(&inv.program);
        cmd.args(&inv.args)
            .envs(inv.env.iter().map(|(k, v)| (k, v)));
        let out = cmd.output().unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// What the program sees as its command line tail: on Windows `echo`
    /// prints it verbatim (MSVCRT-quoted), on Unix `printf` prints the argv.
    fn echo_script(name: &str) -> String {
        if cfg!(windows) {
            format!("echo {}", quote_native(name))
        } else {
            format!("printf '%s' {}", quote_native(name))
        }
    }

    fn expected(name: &str) -> String {
        if cfg!(windows) {
            format!("\"{}\"", name.replace('"', "\\\""))
        } else {
            name.to_string()
        }
    }

    const NAMES: &[&str] = &[
        "%PATH%.txt",
        "%PAIREE_QUOTE_TEST%.txt",
        "a&b.txt",
        "x\"y",
        "100%^!.txt",
        "a b (1) <2> | 3.txt",
        "it's;$(id)`id`.txt",
    ];

    #[test]
    fn std_shell_passes_names_literally() {
        for name in NAMES {
            assert_eq!(run_std(&echo_script(name)), expected(name), "{name}");
        }
    }

    #[test]
    fn invocation_passes_names_literally() {
        for name in NAMES {
            assert_eq!(run_invocation(&echo_script(name)), expected(name), "{name}");
        }
    }

    #[test]
    fn user_variables_and_operators_still_work() {
        let script = if cfg!(windows) {
            "echo %PAIREE_QUOTE_TEST%& echo two"
        } else {
            "echo $PAIREE_QUOTE_TEST; echo two"
        };
        let out = run_std(script);
        assert!(out.starts_with("EXPANDED"), "{out}");
        assert!(out.ends_with("two"), "{out}");
    }

    #[cfg(windows)]
    #[test]
    fn invocation_expands_user_variables_once() {
        let script = "echo %ComSpec%& echo two";
        let out = run_invocation(script);
        assert!(out.to_ascii_lowercase().contains("cmd.exe"), "{out}");
        assert!(out.ends_with("two"), "{out}");
    }
}
