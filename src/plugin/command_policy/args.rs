//! Arguments that turn an allowed tool into a way to run other programs.
//!
//! Secure Mode lets a plugin run the tools it declares, but several common
//! tools execute commands named in their **arguments** (`git -c
//! core.sshCommand=...`, `tar --to-command=...`, `fd --exec`, ...). Each
//! tool has one row in [`TOOL_RULES`]; [`check`] refuses the arguments a
//! row names. Tools that are never allowed (shells, `ssh`, `rsync`,
//! `find`, ...) are in the deny list instead (`deny.rs`).
//!
//! Long options also match their abbreviations (GNU `getopt` and Git accept
//! any unambiguous prefix) and the `--opt=value` form; short options also
//! match inside a bundle (`-xvI prog`) and with an attached value.

/// Arguments of one tool that lead to code execution.
struct ToolRule {
    /// Normalized program names the row applies to.
    tools: &'static [&'static str],
    /// Long options (`--to-command`), refused in any position.
    long: &'static [&'static str],
    /// Short option letters (`I` for `-I`), refused alone or in a bundle.
    short: &'static [char],
    /// Whole arguments (or `arg=` prefixes) refused as they are (`-TT`).
    exact: &'static [&'static str],
    /// Long options whose value may not contain these words (fzf `--bind`
    /// actions that run commands).
    value_words: &'static [(&'static str, &'static [&'static str])],
    /// Arguments starting with one of these are refused (`ext::` URLs,
    /// `less` `+` commands).
    arg_prefixes: &'static [&'static str],
    /// The first argument is an option bundle without `-` (`tar xIf ...`).
    bundle_first: bool,
    /// Options that set configuration (`git -c key=value`); the key is
    /// checked against [`GIT_CONFIG`].
    config_options: &'static [&'static str],
    /// Options refused only for these subcommands (Git `-u` means
    /// `--upload-pack` for `clone` and `ls-remote` only).
    scoped: &'static [(&'static [&'static str], &'static str)],
}

impl ToolRule {
    const EMPTY: ToolRule = ToolRule {
        tools: &[],
        long: &[],
        short: &[],
        exact: &[],
        value_words: &[],
        arg_prefixes: &[],
        bundle_first: false,
        config_options: &[],
        scoped: &[],
    };
}

const TOOL_RULES: &[ToolRule] = &[
    ToolRule {
        tools: &["git"],
        long: &[
            "--upload-pack",
            "--receive-pack",
            "--exec",
            "--exec-path",
            "--template",
            "--ext-diff",
            "--output",
        ],
        arg_prefixes: &["ext::", "fd::"],
        config_options: &["-c", "--config", "--config-env"],
        scoped: &[(&["clone", "ls-remote"], "-u")],
        ..ToolRule::EMPTY
    },
    ToolRule {
        tools: &["tar", "gtar", "bsdtar"],
        long: &[
            "--to-command",
            "--use-compress-program",
            "--checkpoint-action",
            "--info-script",
            "--new-volume-script",
            "--rsh-command",
            "--rmt-command",
        ],
        short: &['I', 'F'],
        bundle_first: true,
        ..ToolRule::EMPTY
    },
    ToolRule {
        tools: &["zip"],
        long: &["--unzip-command"],
        exact: &["-TT"],
        ..ToolRule::EMPTY
    },
    ToolRule {
        tools: &["rg", "ripgrep"],
        long: &["--pre"],
        ..ToolRule::EMPTY
    },
    ToolRule {
        tools: &["fd", "fdfind"],
        long: &["--exec", "--exec-batch"],
        short: &['x', 'X'],
        ..ToolRule::EMPTY
    },
    ToolRule {
        tools: &["fzf"],
        long: &["--preview", "--listen", "--listen-unsafe", "--with-shell"],
        value_words: &[(
            "--bind",
            &[
                "execute",
                "become",
                "reload",
                "transform",
                "preview(",
                "preview:",
            ],
        )],
        ..ToolRule::EMPTY
    },
    ToolRule {
        tools: &["less"],
        long: &["--lesskey-src", "--lesskey-file"],
        short: &['k'],
        arg_prefixes: &["+"],
        ..ToolRule::EMPTY
    },
];

/// Git configuration that runs programs: exact keys, key prefixes and the
/// last components of keys (`diff.<driver>.textconv`).
struct ConfigRules {
    exact: &'static [&'static str],
    prefixes: &'static [&'static str],
    suffixes: &'static [&'static str],
}

const GIT_CONFIG: ConfigRules = ConfigRules {
    exact: &[
        "core.sshcommand",
        "core.pager",
        "core.editor",
        "core.fsmonitor",
        "core.hookspath",
        "core.gitproxy",
        "core.askpass",
        "core.alternaterefscommand",
        "core.worktree",
        "diff.external",
        "sequence.editor",
        "ssh.variant",
        "include.path",
        "web.browser",
        "uploadpack.packobjectshook",
    ],
    prefixes: &[
        "alias.",
        "pager.",
        "filter.",
        "includeif.",
        "credential.",
        "protocol.",
        "url.",
        "gpg.",
        "sendemail.",
    ],
    suffixes: &[
        ".textconv",
        ".command",
        ".driver",
        ".cmd",
        ".helper",
        ".uploadpack",
        ".receivepack",
        ".program",
        ".proxycommand",
        ".sshcommand",
    ],
};

/// Refuses `args` when they make the (normalized) `tool` run another
/// program. Tools without a row pass.
pub fn check(tool: &str, args: &[String]) -> Result<(), String> {
    let Some(rule) = TOOL_RULES.iter().find(|r| r.tools.contains(&tool)) else {
        return Ok(());
    };
    let subcommand = first_operand(args, rule);
    for (i, arg) in args.iter().enumerate() {
        let next = args.get(i + 1).map(String::as_str);
        if let Some(reason) = refused(rule, arg, next, i == 0, subcommand) {
            return Err(format!(
                "Security violation: '{tool} {arg}' can run other programs ({reason}) and is not allowed in Secure Mode"
            ));
        }
    }
    Ok(())
}

/// Why `arg` is refused, if it is.
fn refused(
    rule: &ToolRule,
    arg: &str,
    next: Option<&str>,
    first: bool,
    subcommand: Option<&str>,
) -> Option<&'static str> {
    if rule.long.iter().any(|opt| long_matches(arg, opt)) {
        return Some("option");
    }
    if rule
        .exact
        .iter()
        .any(|e| arg == *e || arg.starts_with(&format!("{e}=")))
    {
        return Some("option");
    }
    if rule.arg_prefixes.iter().any(|p| arg.starts_with(p)) {
        return Some("argument");
    }
    if short_in_bundle(arg, rule.short)
        || (first && rule.bundle_first && bare_bundle(arg, rule.short))
    {
        return Some("option");
    }
    for (opt, words) in rule.value_words {
        if long_matches(arg, opt) {
            let value = arg.split_once('=').map(|(_, v)| v).or(next).unwrap_or("");
            if words.iter().any(|w| value.to_ascii_lowercase().contains(w)) {
                return Some("binding");
            }
        }
    }
    for (subcommands, opt) in rule.scoped {
        if subcommand.is_some_and(|s| subcommands.contains(&s)) && option_is(arg, opt) {
            return Some("option");
        }
    }
    config_value(rule, arg, next)
        .filter(|kv| config_runs_code(kv))
        .map(|_| "configuration")
}

/// `--name`, `--name=value` or any abbreviation (`--na`).
fn long_matches(arg: &str, opt: &str) -> bool {
    let name = arg.split_once('=').map_or(arg, |(n, _)| n);
    name == opt || (name.len() >= 3 && name.starts_with("--") && opt.starts_with(name))
}

/// `-o`, `-ovalue` or `--opt` / `--opt=value` exactly.
fn option_is(arg: &str, opt: &str) -> bool {
    if opt.starts_with("--") {
        long_matches(arg, opt)
    } else {
        arg == opt || (arg.starts_with(opt) && !arg.starts_with("--"))
    }
}

/// A short-option bundle (`-xvI`) containing one of `letters`.
fn short_in_bundle(arg: &str, letters: &[char]) -> bool {
    arg.len() > 1
        && arg.starts_with('-')
        && !arg.starts_with("--")
        && arg[1..].chars().any(|c| letters.contains(&c))
}

/// A traditional bundle without `-` (`tar xIf`).
fn bare_bundle(arg: &str, letters: &[char]) -> bool {
    !arg.starts_with('-')
        && arg.chars().all(|c| c.is_ascii_alphabetic())
        && arg.chars().any(|c| letters.contains(&c))
}

/// The `key=value` (or `key`) set by `arg` when it is a config option.
fn config_value<'a>(rule: &ToolRule, arg: &'a str, next: Option<&'a str>) -> Option<&'a str> {
    rule.config_options.iter().find_map(|opt| {
        if arg == *opt {
            next
        } else if let Some(rest) = arg.strip_prefix(&format!("{opt}=")) {
            Some(rest)
        } else if !opt.starts_with("--") {
            arg.strip_prefix(opt).filter(|r| !r.is_empty())
        } else {
            None
        }
    })
}

/// Whether the Git config `key[=value]` names a program to run.
fn config_runs_code(key_value: &str) -> bool {
    let key = key_value
        .split_once('=')
        .map_or(key_value, |(k, _)| k)
        .trim()
        .to_ascii_lowercase();
    GIT_CONFIG.exact.contains(&key.as_str())
        || GIT_CONFIG.prefixes.iter().any(|p| key.starts_with(p))
        || GIT_CONFIG.suffixes.iter().any(|s| key.ends_with(s))
}

/// The first argument that is not an option or an option's value: Git's
/// subcommand.
fn first_operand<'a>(args: &'a [String], rule: &ToolRule) -> Option<&'a str> {
    const VALUE_OPTIONS: &[&str] = &["-C", "--git-dir", "--work-tree", "--namespace"];
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if VALUE_OPTIONS.contains(&arg.as_str()) || rule.config_options.contains(&arg.as_str()) {
            skip_next = true;
        } else if !arg.starts_with('-') {
            return Some(arg);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::check;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn exec_vectors_are_refused() {
        let refused: &[(&str, &[&str])] = &[
            ("git", &["-c", "core.sshCommand=touch /tmp/x", "fetch"]),
            ("git", &["-ccore.pager=sh", "log"]),
            ("git", &["-c", "alias.x=!sh", "x"]),
            ("git", &["--config-env=core.editor=EDITOR", "commit"]),
            ("git", &["clone", "--config", "core.fsmonitor=evil", "u"]),
            ("git", &["-c", "diff.foo.textconv=evil", "diff"]),
            ("git", &["-c", "Credential.https://h.Helper=!sh", "push"]),
            ("git", &["fetch", "--upload-pack=evil", "origin"]),
            ("git", &["clone", "--upload-p", "evil", "u"]),
            ("git", &["clone", "-u", "evil", "u"]),
            ("git", &["ls-remote", "-uevil", "u"]),
            ("git", &["push", "--ex", "evil"]),
            ("git", &["push", "--exec=evil"]),
            ("git", &["clone", "ext::sh -c touch% /tmp/x", "d"]),
            ("git", &["clone", "--template=/tmp/hooks", "u"]),
            ("git", &["diff", "--output=/etc/x"]),
            ("tar", &["--to-command=sh", "-xf", "a.tar"]),
            ("tar", &["-xf", "a.tar", "--use-compress-program", "evil"]),
            ("tar", &["--use-comp=evil", "-xf", "a.tar"]),
            ("tar", &["-xvIevil", "-f", "a.tar"]),
            ("tar", &["xIf", "evil", "a.tar"]),
            ("tar", &["--checkpoint-action=exec=sh"]),
            ("bsdtar", &["-F", "script", "-cf", "a.tar"]),
            ("zip", &["-TT", "evil", "a.zip", "f"]),
            ("zip", &["--unzip-command=evil", "a.zip"]),
            ("rg", &["--pre", "evil", "x"]),
            ("rg", &["--pre=evil", "x"]),
            ("fd", &["-x", "rm", "{}"]),
            ("fd", &["--exec-batch", "rm"]),
            ("fd", &["-HX", "rm"]),
            ("fzf", &["--preview", "cat {}"]),
            ("fzf", &["--bind", "enter:execute(sh)"]),
            ("fzf", &["--bind=ctrl-r:reload(sh)"]),
            ("less", &["+!sh", "f"]),
            ("less", &["--lesskey-src=f", "f"]),
        ];
        for (tool, list) in refused {
            let err = check(tool, &args(list)).unwrap_err();
            assert!(
                err.starts_with("Security violation"),
                "{tool} {list:?}: {err}"
            );
        }
    }

    #[test]
    fn ordinary_uses_pass() {
        let allowed: &[(&str, &[&str])] = &[
            ("git", &["status", "--porcelain"]),
            ("git", &["-c", "color.ui=always", "log", "--oneline"]),
            ("git", &["-C", "/repo", "diff", "--stat"]),
            ("git", &["add", "-u"]),
            ("git", &["push", "-u", "origin", "main"]),
            ("git", &["fetch", "-u", "origin"]),
            ("git", &["format-patch", "--output-directory", "out"]),
            ("git", &["commit", "-c", "HEAD"]),
            ("git", &["log", "--format=%H"]),
            ("tar", &["-xzf", "a.tar.gz", "-C", "out"]),
            ("tar", &["xzf", "a.tar.gz"]),
            ("tar", &["--list", "-f", "a.tar"]),
            ("zip", &["-r", "a.zip", "dir"]),
            ("rg", &["--json", "-n", "pattern"]),
            ("fd", &["-H", "-e", "rs"]),
            ("fzf", &["--bind", "ctrl-a:select-all", "--height", "40%"]),
            ("fzf", &["--bind=?:toggle-preview", "--preview-window=up"]),
            ("rg", &["--pre-glob", "*.pdf", "x"]),
            ("less", &["-R", "file"]),
            ("cargo", &["--version"]),
        ];
        for (tool, list) in allowed {
            assert!(check(tool, &args(list)).is_ok(), "{tool} {list:?}");
        }
    }
}
