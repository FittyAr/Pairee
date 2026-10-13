//! `--help` / `-h` output of the binary and of its subcommands.
//!
//! Every text comes from the translation files; the tables below only hold
//! the literal option and command names and the keys of their descriptions.

use crate::config::localization::t;

/// What a help page describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    Main,
    Plugin,
    Developer,
    Keymap,
}

/// One row: the literal name, the key of its argument placeholder (if any)
/// and the key of its description.
type Row = (&'static str, Option<&'static str>, &'static str);

const MAIN_OPTIONS: &[Row] = &[
    (
        "--cwd-file",
        Some("cli_help_arg_file"),
        "cli_help_opt_cwd_file",
    ),
    ("--print-cwd", None, "cli_help_opt_print_cwd"),
    ("--no-mouse", None, "cli_help_opt_no_mouse"),
    ("-h, --help", None, "cli_help_opt_help"),
    ("-V, --version", None, "cli_help_opt_version"),
];

const MAIN_COMMANDS: &[Row] = &[
    ("plugin", None, "cli_help_cmd_plugin"),
    ("developer", None, "cli_help_cmd_developer"),
    ("keymap", None, "cli_help_cmd_keymap"),
];

const PLUGIN_COMMANDS: &[Row] = &[
    ("list", None, "cli_help_plugin_list"),
    (
        "search",
        Some("cli_help_arg_query"),
        "cli_help_plugin_search",
    ),
    ("info", Some("cli_help_arg_name"), "cli_help_plugin_info"),
    (
        "install",
        Some("cli_help_arg_specs"),
        "cli_help_plugin_install",
    ),
    (
        "remove",
        Some("cli_help_arg_name"),
        "cli_help_plugin_remove",
    ),
    ("pin", Some("cli_help_arg_name"), "cli_help_plugin_pin"),
    ("unpin", Some("cli_help_arg_name"), "cli_help_plugin_unpin"),
    ("verify", None, "cli_help_plugin_verify"),
    ("check-updates", None, "cli_help_plugin_check_updates"),
    (
        "update",
        Some("cli_help_arg_optional_name"),
        "cli_help_plugin_update",
    ),
];

const DEVELOPER_COMMANDS: &[Row] = &[
    ("init", Some("cli_help_arg_name"), "cli_help_dev_init"),
    ("lint", None, "cli_help_dev_lint"),
    ("package", None, "cli_help_dev_package"),
    ("submit", None, "cli_help_dev_submit"),
];

const KEYMAP_COMMANDS: &[Row] = &[(
    "print",
    Some("cli_help_arg_keymap_print"),
    "cli_help_keymap_print",
)];

/// `true` when `args` ask for help (`--help` or `-h` before a `--`).
pub fn requested(args: &[String]) -> bool {
    args.iter()
        .take_while(|a| *a != "--")
        .any(|a| a == "--help" || a == "-h")
}

/// The first argument, unless it asks for help.
pub fn command_of(args: &[String]) -> Option<&str> {
    if requested(args) {
        return None;
    }
    args.first().map(String::as_str)
}

/// The localized help page of `topic`.
pub fn text(topic: Topic) -> String {
    let mut out = String::new();
    if topic == Topic::Main {
        out.push_str(&format!(
            "pairee {} - {}\n\n",
            env!("CARGO_PKG_VERSION"),
            t("cli_help_about")
        ));
    }
    out.push_str(&t("cli_help_usage"));
    out.push('\n');
    for line in t(usage_key(topic)).lines() {
        out.push_str(&format!("  {line}\n"));
    }
    if topic == Topic::Main {
        section(&mut out, "cli_help_options", MAIN_OPTIONS);
    }
    section(&mut out, "cli_help_commands", commands(topic));
    out.push('\n');
    out.push_str(&t(match topic {
        Topic::Main => "cli_help_more",
        _ => "cli_help_more_main",
    }));
    out.push('\n');
    out
}

/// Message for a `command` given without its required argument, followed
/// by the help page of `topic`.
pub fn missing_argument(topic: Topic, command: &str) -> String {
    let arg = commands(topic)
        .iter()
        .find(|(name, _, _)| *name == command)
        .and_then(|(_, arg, _)| *arg)
        .map(t)
        .unwrap_or_default();
    let full = format!("pairee {} {command}", name(topic));
    let message = t("cli_error_missing_argument")
        .replacen("{}", &full, 1)
        .replacen("{}", &arg, 1);
    format!("{message}\n\n{}", text(topic))
}

/// Message for an unknown `command`, followed by the help page of `topic`.
pub fn unknown_command(topic: Topic, command: &str) -> String {
    let message = t("cli_error_unknown_command").replacen("{}", command, 1);
    format!("{message}\n\n{}", text(topic))
}

/// The subcommand of `topic` as typed on the command line.
fn name(topic: Topic) -> &'static str {
    match topic {
        Topic::Main => "",
        Topic::Plugin => "plugin",
        Topic::Developer => "developer",
        Topic::Keymap => "keymap",
    }
}

fn usage_key(topic: Topic) -> &'static str {
    match topic {
        Topic::Main => "cli_help_usage_main",
        Topic::Plugin => "cli_help_usage_plugin",
        Topic::Developer => "cli_help_usage_developer",
        Topic::Keymap => "cli_help_usage_keymap",
    }
}

fn commands(topic: Topic) -> &'static [Row] {
    match topic {
        Topic::Main => MAIN_COMMANDS,
        Topic::Plugin => PLUGIN_COMMANDS,
        Topic::Developer => DEVELOPER_COMMANDS,
        Topic::Keymap => KEYMAP_COMMANDS,
    }
}

/// Appends a titled, aligned two-column list of `rows`.
fn section(out: &mut String, title_key: &str, rows: &[Row]) {
    let names: Vec<String> = rows
        .iter()
        .map(|(name, arg, _)| match arg {
            Some(key) => format!("{name} {}", t(key)),
            None => (*name).to_string(),
        })
        .collect();
    let width = names
        .iter()
        .map(|n| unicode_width::UnicodeWidthStr::width(n.as_str()))
        .max()
        .unwrap_or(0);
    out.push('\n');
    out.push_str(&t(title_key));
    out.push('\n');
    for (name, (_, _, desc_key)) in names.iter().zip(rows) {
        let pad = width - unicode_width::UnicodeWidthStr::width(name.as_str());
        out.push_str(&format!("  {name}{}  {}\n", " ".repeat(pad), t(desc_key)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn help_flags_are_recognized_before_double_dash() {
        assert!(requested(&args(&["--help"])));
        assert!(requested(&args(&["/a", "-h"])));
        assert!(!requested(&args(&["--", "-h"])));
        assert!(!requested(&args(&["/a"])));
        assert_eq!(command_of(&args(&["list"])), Some("list"));
        assert_eq!(command_of(&args(&["install", "--help"])), None);
    }

    #[test]
    fn every_page_is_fully_translated() {
        for topic in [Topic::Main, Topic::Plugin, Topic::Developer, Topic::Keymap] {
            let page = text(topic);
            assert!(
                !page.contains("cli_"),
                "untranslated key in {topic:?}:\n{page}"
            );
        }
        let page = missing_argument(Topic::Plugin, "info");
        assert!(page.starts_with("pairee plugin info"), "{page}");
        assert!(!page.contains("cli_"), "{page}");
        let page = unknown_command(Topic::Developer, "bogus");
        assert!(page.contains("bogus") && !page.contains("cli_"), "{page}");
    }

    #[test]
    fn main_page_lists_options_and_commands() {
        let page = text(Topic::Main);
        for word in ["--cwd-file", "--no-mouse", "--version", "plugin", "keymap"] {
            assert!(page.contains(word), "{word} missing:\n{page}");
        }
    }
}
