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
    /// Options (or operands) refused only for these subcommands (Git `-u`
    /// means `--upload-pack` for `clone` and `ls-remote` only; `submodule
    /// foreach` runs a shell command).
    scoped: &'static [(&'static [&'static str], &'static str)],
    /// Subcommands refused outright (`git filter-branch` runs its filters
    /// through the shell).
    subcommands: &'static [&'static str],
    /// The subcommand that writes configuration (`git config`): operands
    /// naming a key that runs programs are refused, so the plugin cannot
    /// store `core.fsmonitor` or an alias in a repository's config and
    /// trigger it with the next command.
    config_subcommand: Option<&'static str>,
    /// Arguments put before the plugin's own (Git: no implicit bare
    /// repositories, whose `config` a plugin could have written with
    /// `pairee.fs`).
    prepend: &'static [&'static str],
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
        subcommands: &[],
        config_subcommand: None,
        prepend: &[],
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
            "--git-dir",
        ],
        arg_prefixes: &["ext::", "fd::"],
        config_options: &["-c", "--config", "--config-env"],
        scoped: &[
            (&["clone", "ls-remote"], "-u"),
            (&["rebase"], "-x"),
            (&["difftool"], "-x"),
            (&["difftool"], "--extcmd"),
            (&["grep"], "-O"),
            (&["grep"], "--open-files-in-pager"),
            (&["submodule"], "foreach"),
            (&["bisect"], "run"),
            (&["config"], "--rename-section"),
        ],
        subcommands: &["filter-branch", "send-email", "instaweb", "daemon"],
        config_subcommand: Some("config"),
        prepend: &["-c", "safe.bareRepository=explicit"],
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
        "difftool.",
        "mergetool.",
        "safe.",
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

/// What a command does to a path named in its arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathUse {
    Read,
    Write,
}

/// Paths that `tool` writes (or, for `zip`, packs) according to `args`:
/// where `tar` extracts / creates, where `git` works or clones to, the
/// archive `zip` creates. The caller checks each against the plugin's
/// write jail, so an allowed tool cannot overwrite what `pairee.fs` could
/// not (Pairee's own config, other plugins). This is a best-effort list for
/// the common options of the few tools Secure Mode lets through; the
/// working directory is checked separately. Tools without rows have none.
pub fn path_arguments(tool: &str, args: &[String]) -> Vec<(String, PathUse)> {
    match tool {
        "tar" | "gtar" | "bsdtar" => tar_paths(args),
        "git" => git_paths(args),
        "zip" => zip_paths(args),
        _ => Vec::new(),
    }
}

/// The value of option `short` / `long` at `args[i]`: `-C dir`, `-Cdir`,
/// `--directory dir`, `--directory=dir`, and a bundle ending in the letter
/// (`-xzf a.tar`).
fn option_value<'a>(args: &'a [String], i: usize, short: char, long: &str) -> Option<&'a str> {
    let arg = args[i].as_str();
    let next = args.get(i + 1).map(String::as_str);
    if let Some(rest) = arg.strip_prefix("--") {
        let (name, value) = rest
            .split_once('=')
            .map_or((rest, None), |(n, v)| (n, Some(v)));
        let name = format!("--{name}");
        return (name.len() >= 3 && long.starts_with(&name)).then(|| value.or(next))?;
    }
    let bundle = arg.strip_prefix('-')?;
    let at = bundle.find(short)?;
    let rest = &bundle[at + short.len_utf8()..];
    if rest.is_empty() { next } else { Some(rest) }
}

fn tar_paths(args: &[String]) -> Vec<(String, PathUse)> {
    let mut creating = false;
    let mut files = Vec::new();
    let mut paths = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        let bare = i == 0 && !arg.starts_with('-');
        if bare || (arg.starts_with('-') && !arg.starts_with("--")) {
            let letters = arg.trim_start_matches('-');
            // Letters after `f` / `C` are that option's value (`-Cout`).
            creating |= letters
                .chars()
                .take_while(|c| !matches!(c, 'f' | 'C'))
                .any(|c| matches!(c, 'c' | 'r' | 'u'));
        } else if ["--create", "--append", "--update"]
            .iter()
            .any(|o| long_matches(arg, o))
        {
            creating = true;
        }
        let value = if bare {
            // `tar xzf a.tar`: the operand after the bundle belongs to `f`.
            arg.contains('f')
                .then(|| args.get(i + 1).map(String::as_str))
                .flatten()
        } else {
            option_value(args, i, 'f', "--file")
        };
        if let Some(file) = value
            && (bare || arg.starts_with('-'))
        {
            files.push(file.to_string());
        }
        if !bare && let Some(dir) = option_value(args, i, 'C', "--directory") {
            paths.push((dir.to_string(), PathUse::Write));
        }
    }
    let file_use = if creating {
        PathUse::Write
    } else {
        PathUse::Read
    };
    paths.extend(files.into_iter().map(|f| (f, file_use)));
    paths
}

fn git_paths(args: &[String]) -> Vec<(String, PathUse)> {
    const VALUE_OPTIONS: &[&str] = &[
        "-b",
        "--branch",
        "--depth",
        "-o",
        "--origin",
        "--reference",
        "--filter",
        "-j",
        "--jobs",
        "-c",
        "--config",
        "--shallow-since",
        "--shallow-exclude",
    ];
    let mut paths = Vec::new();
    let mut operands: Vec<&str> = Vec::new();
    let mut subcommand_seen = false;
    let mut skip = false;
    for (i, arg) in args.iter().enumerate() {
        if skip {
            skip = false;
            continue;
        }
        if !subcommand_seen {
            // Global options: `-C <dir>` / `--work-tree <dir>` move the work.
            if let Some(dir) = option_value(args, i, 'C', "--work-tree")
                .filter(|_| arg.starts_with("-C") || arg.starts_with("--work"))
            {
                paths.push((dir.to_string(), PathUse::Write));
                skip = arg == "-C" || arg == "--work-tree";
            } else if arg == "-c" || arg == "--config-env" {
                skip = true;
            } else if !arg.starts_with('-') {
                subcommand_seen = true;
                operands.push(arg);
            }
            continue;
        }
        if let Some(dir) = arg.strip_prefix("--separate-git-dir=") {
            paths.push((dir.to_string(), PathUse::Write));
        } else if arg == "--separate-git-dir" {
            paths.extend(args.get(i + 1).map(|d| (d.clone(), PathUse::Write)));
            skip = true;
        } else if VALUE_OPTIONS.contains(&arg.as_str()) {
            skip = true;
        } else if !arg.starts_with('-') {
            operands.push(arg);
        }
    }
    // operands[0] is the subcommand.
    let destination = match operands.as_slice() {
        ["clone", _url, dest, ..] => Some(*dest),
        ["init", dest, ..] => Some(*dest),
        ["worktree", "add", path, ..] => Some(*path),
        _ => None,
    };
    paths.extend(destination.map(|d| (d.to_string(), PathUse::Write)));
    paths
}

fn zip_paths(args: &[String]) -> Vec<(String, PathUse)> {
    let mut paths = Vec::new();
    let mut operands = Vec::new();
    let mut skip = false;
    for (i, arg) in args.iter().enumerate() {
        if skip {
            skip = false;
            continue;
        }
        if let Some(path) = arg.strip_prefix("--output-file=") {
            paths.push((path.to_string(), PathUse::Write));
        } else if arg == "--output-file" || arg == "-O" || arg == "-b" || arg == "--temp-path" {
            paths.extend(args.get(i + 1).map(|p| (p.clone(), PathUse::Write)));
            skip = true;
        } else if !arg.starts_with('-') {
            operands.push(arg);
        }
    }
    // First operand: the archive; the rest are files to pack.
    for (n, operand) in operands.into_iter().enumerate() {
        let usage = if n == 0 {
            PathUse::Write
        } else {
            PathUse::Read
        };
        paths.push((operand.clone(), usage));
    }
    paths
}

/// Arguments Secure Mode puts before the plugin's own for `tool`.
pub fn prepended(tool: &str) -> &'static [&'static str] {
    rule_of(tool).map_or(&[], |rule| rule.prepend)
}

fn rule_of(tool: &str) -> Option<&'static ToolRule> {
    TOOL_RULES.iter().find(|r| r.tools.contains(&tool))
}

/// Refuses `args` when they make the (normalized) `tool` run another
/// program. Tools without a row pass.
pub fn check(tool: &str, args: &[String]) -> Result<(), String> {
    let Some(rule) = rule_of(tool) else {
        return Ok(());
    };
    let subcommand = first_operand(args, rule);
    if let Some(sub) = subcommand.filter(|s| rule.subcommands.contains(s)) {
        return Err(format!(
            "Security violation: '{tool} {sub}' can run other programs and is not allowed in Secure Mode"
        ));
    }
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
        if subcommand.is_some_and(|s| subcommands.contains(&s)) && scoped_matches(arg, opt) {
            return Some("option");
        }
    }
    if rule.config_subcommand.is_some() && subcommand == rule.config_subcommand {
        let operand = !arg.starts_with('-') && Some(arg) != subcommand;
        if operand && config_runs_code(arg) {
            return Some("configuration");
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

/// `opt` as [`option_is`], a single-letter option also inside a bundle
/// (`git rebase -ix cmd`).
fn scoped_matches(arg: &str, opt: &str) -> bool {
    let mut letters = opt.strip_prefix('-').unwrap_or("").chars();
    let short = match (letters.next(), letters.next()) {
        (Some(c), None) if c != '-' => Some(c),
        _ => None,
    };
    option_is(arg, opt) || short.is_some_and(|c| short_in_bundle(arg, &[c]))
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
            ("git", &["config", "core.fsmonitor", "evil"]),
            ("git", &["config", "--local", "alias.x", "!sh"]),
            ("git", &["config", "set", "Core.Pager", "sh"]),
            (
                "git",
                &["-C", "r", "config", "--add", "filter.x.clean", "evil"],
            ),
            ("git", &["config", "--rename-section", "x", "core"]),
            ("git", &["-c", "safe.bareRepository=all", "status"]),
            ("git", &["--git-dir=data/evil", "status"]),
            ("git", &["rebase", "-x", "evil", "HEAD~1"]),
            ("git", &["rebase", "-ix", "evil", "HEAD~1"]),
            ("git", &["difftool", "--extcmd=evil"]),
            ("git", &["grep", "-Oevil", "x"]),
            ("git", &["submodule", "foreach", "evil"]),
            ("git", &["bisect", "run", "evil"]),
            ("git", &["filter-branch", "--tree-filter", "evil"]),
            ("git", &["send-email", "--sendmail-cmd=evil"]),
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
    fn written_paths_are_found_in_common_forms() {
        use super::{PathUse::*, path_arguments};
        let paths = |tool: &str, list: &[&str]| path_arguments(tool, &args(list));
        let w = |p: &str| (p.to_string(), Write);
        let r = |p: &str| (p.to_string(), Read);
        assert_eq!(
            paths("tar", &["-xzf", "a.tgz", "-C", "out"]),
            [w("out"), r("a.tgz")]
        );
        assert_eq!(
            paths("tar", &["xf", "a.tar", "-Cout"]),
            [w("out"), r("a.tar")]
        );
        assert_eq!(
            paths("tar", &["--directory=out", "-xf", "a"]),
            [w("out"), r("a")]
        );
        assert_eq!(paths("tar", &["-czf", "../x.tgz", "d"]), [w("../x.tgz")]);
        assert_eq!(paths("tar", &["cf", "x.tar", "d"]), [w("x.tar")]);
        assert_eq!(paths("git", &["-C", "repo", "status"]), [w("repo")]);
        assert_eq!(paths("git", &["--work-tree=wt", "status"]), [w("wt")]);
        assert_eq!(
            paths("git", &["clone", "--depth", "1", "https://h/r.git", "dest"]),
            [w("dest")]
        );
        assert_eq!(paths("git", &["init", "newrepo"]), [w("newrepo")]);
        assert_eq!(
            paths("git", &["clone", "--separate-git-dir", "g", "u", "d"]),
            [w("g"), w("d")]
        );
        assert!(paths("git", &["log", "--oneline"]).is_empty());
        assert_eq!(
            paths("zip", &["-r", "out.zip", "dir"]),
            [w("out.zip"), r("dir")]
        );
        assert!(paths("rg", &["x", "."]).is_empty());
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
            ("git", &["config", "--get", "user.name"]),
            ("git", &["config", "user.email", "me@example.com"]),
            ("git", &["submodule", "status"]),
            ("git", &["rebase", "--continue"]),
            ("git", &["grep", "-n", "x"]),
        ];
        for (tool, list) in allowed {
            assert!(check(tool, &args(list)).is_ok(), "{tool} {list:?}");
        }
    }
}
