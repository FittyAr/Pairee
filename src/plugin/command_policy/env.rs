//! Environment variables a Secure Mode plugin may set on the commands it
//! spawns (`Command:env(name, value)`).
//!
//! Many tools read the environment to run other programs or load code
//! (`GIT_SSH_COMMAND`, `GIT_EXTERNAL_DIFF`, `LD_PRELOAD`, `DYLD_*`, `PAGER`,
//! `EDITOR`, `BASH_ENV`, `PYTHONPATH`, `NODE_OPTIONS`, `PERL5OPT`,
//! `RUBYOPT`, ...). A denylist can never be complete, so Secure Mode uses an
//! **allowlist**: only locale, terminal and colour variables, which change
//! how a tool formats its output but not what it runs. The user's own
//! environment is inherited unchanged (the plugin cannot alter it).

/// Exact names a Secure Mode plugin may set.
const ALLOWED: &[&str] = &[
    "LANG",
    "LANGUAGE",
    "TERM",
    "COLORTERM",
    "NO_COLOR",
    "CLICOLOR",
    "CLICOLOR_FORCE",
    "FORCE_COLOR",
    "TZ",
    "COLUMNS",
    "LINES",
];

/// `true` when a Secure Mode plugin may set `name` (`LC_*` included).
pub fn is_allowed(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    let lc_var = upper
        .strip_prefix("LC_")
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_uppercase()));
    lc_var || ALLOWED.contains(&upper.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_and_colour_variables_are_allowed() {
        for name in ["LANG", "LC_ALL", "lc_messages", "NO_COLOR", "TERM", "TZ"] {
            assert!(is_allowed(name), "{name}");
        }
    }

    #[test]
    fn variables_that_run_code_are_refused() {
        for name in [
            "GIT_SSH_COMMAND",
            "GIT_EXTERNAL_DIFF",
            "GIT_CONFIG_GLOBAL",
            "LD_PRELOAD",
            "LD_LIBRARY_PATH",
            "DYLD_INSERT_LIBRARIES",
            "PAGER",
            "EDITOR",
            "VISUAL",
            "BASH_ENV",
            "ENV",
            "PROMPT_COMMAND",
            "PYTHONPATH",
            "PYTHONSTARTUP",
            "NODE_OPTIONS",
            "PERL5OPT",
            "PERL5LIB",
            "RUBYOPT",
            "PATH",
            "HOME",
            "LC_",
            "LC_ALL=x",
            "",
        ] {
            assert!(!is_allowed(name), "{name}");
        }
    }
}
