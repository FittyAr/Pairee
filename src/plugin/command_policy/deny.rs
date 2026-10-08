//! Hard deny list applied on top of the Secure Mode allowlist.
//!
//! A plugin cannot spawn these even when it declares them in its manifest:
//! shells, script interpreters, LOLBins, network clients and wrappers that
//! execute their arguments as another command. Names are normalized first so
//! `CMD.EXE`, `bash.cmd` or `python3.12` match their base names.

const DENIED_COMMANDS: &[&str] = &[
    // Network clients
    "curl",
    "wget",
    "nc",
    "ncat",
    "netcat",
    "socat",
    "ssh",
    "scp",
    "sftp",
    "telnet",
    "ftp",
    "tftp",
    "rsync",
    "nmap",
    "bitsadmin",
    "certutil",
    // Unix shells and multi-call binaries
    "sh",
    "bash",
    "zsh",
    "csh",
    "tcsh",
    "ksh",
    "mksh",
    "dash",
    "ash",
    "fish",
    "busybox",
    "toybox",
    // Windows shells and script hosts
    "cmd",
    "powershell",
    "powershell_ise",
    "pwsh",
    "wscript",
    "cscript",
    "mshta",
    "rundll32",
    "regsvr32",
    "msiexec",
    "wmic",
    "schtasks",
    "forfiles",
    "msbuild",
    "installutil",
    "conhost",
    "explorer",
    // Script interpreters / runtimes
    "python",
    "pythonw",
    "py",
    "pyw",
    "perl",
    "ruby",
    "irb",
    "node",
    "nodejs",
    "deno",
    "bun",
    "php",
    "lua",
    "luajit",
    "tclsh",
    "wish",
    "expect",
    "java",
    "jshell",
    "osascript",
    "awk",
    "gawk",
    "mawk",
    "nawk",
    // Wrappers that execute their arguments as a command
    "env",
    "xargs",
    "find",
    "nohup",
    "nice",
    "timeout",
    "setsid",
    "stdbuf",
    "script",
    "sudo",
    "su",
    "doas",
    "pkexec",
    "runas",
    "start",
    "open",
];

/// Prefixes that are always denied (versioned interpreters such as
/// `python3.12`, `powershell_ise`, `perl5.36`).
const DENIED_PREFIXES: &[&str] = &[
    "python",
    "powershell",
    "pwsh",
    "perl",
    "ruby",
    "php",
    "node",
    "lua",
];

/// Extensions Windows resolves implicitly; stripped before matching.
const EXECUTABLE_EXTENSIONS: &[&str] = &[
    ".exe", ".com", ".cmd", ".bat", ".ps1", ".vbs", ".js", ".msc",
];

/// Reduces a program path to a lowercase base name without directory,
/// trailing dots/spaces (ignored by Windows) or executable extension.
pub fn normalize_command_name(cmd: &str) -> String {
    let file = cmd.rsplit(['/', '\\']).next().unwrap_or(cmd);
    let mut name = file
        .trim()
        .trim_end_matches(['.', ' '])
        .to_ascii_lowercase();
    while let Some(ext) = EXECUTABLE_EXTENSIONS
        .iter()
        .find(|ext| name.len() > ext.len() && name.ends_with(*ext))
    {
        name.truncate(name.len() - ext.len());
        name = name.trim_end_matches(['.', ' ']).to_string();
    }
    name
}

/// `true` when the (already normalized) name is a shell, interpreter,
/// network client or command wrapper. Empty names are denied.
pub fn is_denied(normalized: &str) -> bool {
    if normalized.is_empty() {
        return true;
    }
    let unversioned =
        normalized.trim_end_matches(|c: char| c.is_ascii_digit() || matches!(c, '.' | '-' | '_'));
    DENIED_COMMANDS.contains(&normalized)
        || DENIED_COMMANDS.contains(&unversioned)
        || DENIED_PREFIXES.iter().any(|p| normalized.starts_with(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn denied(cmd: &str) -> bool {
        is_denied(&normalize_command_name(cmd))
    }

    #[test]
    fn shells_and_interpreters_are_denied_after_normalization() {
        for blocked in [
            "CMD.EXE",
            "C:\\Windows\\System32\\cmd.exe",
            "/usr/bin/bash",
            "bash.cmd",
            "PowerShell.exe",
            "pwsh",
            "python3.12",
            "pythonw.exe",
            "py.exe",
            "node.bat",
            "wscript.exe",
            "mshta.exe",
            "rundll32.exe",
            "/usr/bin/env",
            "busybox",
            "cmd.",
            "cmd.exe. ",
            "perl5.36",
            "curl",
            "",
        ] {
            assert!(denied(blocked), "{blocked:?} should be denied");
        }
    }

    #[test]
    fn ordinary_tools_are_not_denied() {
        for allowed in ["git", "cargo", "ls", "C:\\tools\\rg.exe", "/usr/bin/fd"] {
            assert!(!denied(allowed), "{allowed:?} should not be denied");
        }
    }
}
