//! Non-interactive subcommands: `--elevated-helper`, `--version`, `--help`,
//! `pairee plugin ...`, `pairee developer ...` and `pairee keymap ...`.
//! Each runs to completion without starting the TUI.

use anyhow::Result;
use std::path::PathBuf;

use super::help::{self, Topic};
use crate::fs;
use crate::plugin;

/// Runs the subcommand named by `args` (the full process arguments).
/// Returns `Ok(true)` when one was handled and the process should exit.
pub(super) async fn run_subcommand(args: &[String]) -> Result<bool> {
    if let Some(pos) = args.iter().position(|a| a == "--elevated-helper") {
        let Some(temp_file) = args.get(pos + 1) else {
            anyhow::bail!("Missing temp file argument for --elevated-helper");
        };
        fs::elevated_helper::run_elevated_helper_loop(&PathBuf::from(temp_file))?;
        return Ok(true);
    }
    let first = args.get(1).map(String::as_str);
    if first == Some("--version") || first == Some("-V") {
        println!("pairee {}", env!("CARGO_PKG_VERSION"));
        return Ok(true);
    }
    let rest = args.get(2..).unwrap_or_default();
    let topic = match first {
        Some("plugin") => Topic::Plugin,
        Some("developer") => Topic::Developer,
        Some("keymap") => Topic::Keymap,
        _ if help::requested(args.get(1..).unwrap_or_default()) => Topic::Main,
        _ => return Ok(false),
    };
    crate::config::load_configured_language();
    let cmd = help::command_of(rest);
    let rest = rest.get(1..).unwrap_or_default();
    match (topic, cmd) {
        (Topic::Plugin, Some(cmd)) => plugin_command(cmd, rest).await?,
        (Topic::Developer, Some(cmd)) => super::dev_cli::developer_command(cmd, rest).await?,
        (Topic::Keymap, Some(cmd)) => keymap_command(cmd, rest),
        // `pairee --help ...`, or a subcommand alone or with `--help`.
        _ => print!("{}", help::text(topic)),
    }
    Ok(true)
}

/// `pairee keymap print [--preset <name>] [--lang <code>]`: the Markdown
/// reference of a shipped preset (the source of `help/<lang>/keymap_*.md`).
fn keymap_command(cmd: &str, rest: &[String]) {
    use crate::keybindings::embedded::{PRESETS, normalize_preset_name, preset_toml};
    let value = |flag: &str| {
        rest.iter()
            .position(|a| a == flag)
            .and_then(|i| rest.get(i + 1))
            .cloned()
    };
    if cmd != "print" {
        print!("{}", help::unknown_command(Topic::Keymap, cmd));
        return;
    }
    let preset = normalize_preset_name(&value("--preset").unwrap_or_else(|| "norton".into()));
    if preset_toml(&preset).is_none() || !PRESETS.iter().any(|(n, _)| *n == preset) {
        let names: Vec<&str> = PRESETS.iter().map(|(n, _)| *n).collect();
        println!(
            "{}",
            crate::config::localization::t("cli_error_unknown_preset")
                .replacen("{}", &preset, 1)
                .replacen("{}", &names.join(", "), 1)
        );
        return;
    }
    if let Some(code) = value("--lang")
        && let Some((name, _)) = crate::config::localization::discovery::discover_languages()
            .into_iter()
            .find(|(_, path)| {
                path.file_stem()
                    .is_some_and(|s| s.eq_ignore_ascii_case(&code))
            })
    {
        crate::config::localization::loader::load_language(&name);
    }
    let text = crate::keybindings::reference::markdown(&preset, &crate::config::localization::t);
    print!("{text}");
}

/// `pairee plugin <cmd> [rest...]`. The updater returns reports; this CLI
/// prints them (the updater itself never writes to stdout).
async fn plugin_command(cmd: &str, rest: &[String]) -> Result<()> {
    use plugin::updater;
    let first = rest.first().map(String::as_str);
    let settings = crate::config::AppConfig::load_or_create()
        .map(|c| c.settings)
        .unwrap_or_default();
    let trust = updater::RegistryTrust::from_settings(&settings);
    let report = match (cmd, first) {
        ("list", _) => updater::list_installed(&settings.plugins, &trust).await?,
        ("search", Some(query)) => updater::search(query, &trust).await?,
        ("info", Some(name)) => updater::show_info(name, &trust).await?,
        ("install" | "add", Some(_)) => install_plugins(rest, &trust).await,
        ("remove", Some(name)) => updater::remove(name)?,
        ("pin", Some(name)) => updater::pin(name, true)?,
        ("unpin", Some(name)) => updater::pin(name, false)?,
        ("verify", _) => updater::verify(&trust).await?,
        ("check-updates", _) => updater::check_updates(&trust).await?,
        ("update", name) => updater::update(name, &trust).await?,
        ("search" | "info" | "remove" | "pin" | "unpin", None) => {
            message(help::missing_argument(Topic::Plugin, cmd))
        }
        ("install" | "add", None) => message(help::missing_argument(Topic::Plugin, "install")),
        _ => message(help::unknown_command(Topic::Plugin, cmd)),
    };
    println!("{report}");
    Ok(())
}

/// A report holding one message.
fn message(text: String) -> plugin::updater::Report {
    let mut report = plugin::updater::Report::default();
    report.lines.push(text.trim_end().to_string());
    report
}

/// Installs each `name[@version]`; a failure is reported and the rest continue.
async fn install_plugins(
    specs: &[String],
    trust: &plugin::updater::RegistryTrust,
) -> plugin::updater::Report {
    let mut report = plugin::updater::Report::default();
    for spec in specs {
        let (name, version) = parse_install_spec(spec);
        match plugin::updater::install(&name, version, trust).await {
            Ok(r) => report.absorb(r),
            Err(e) => report
                .lines
                .push(format!("Error installing plugin '{name}': {e:#}")),
        }
    }
    report
}

/// `foo@1.2` -> (`foo`, `Some("1.2")`). Registry names are bare identifiers,
/// so a `.pairee` suffix (the name of the installed folder) is dropped.
fn parse_install_spec(spec: &str) -> (String, Option<&str>) {
    let (base, version) = match spec.split_once('@') {
        Some((base, version)) => (base, Some(version).filter(|v| !v.is_empty())),
        None => (spec, None),
    };
    let name = base.strip_suffix(".pairee").unwrap_or(base);
    (name.to_string(), version)
}

#[cfg(test)]
mod tests {
    use super::parse_install_spec;
    use crate::plugin::updater::ops::validate_identifier;

    #[test]
    fn install_spec_drops_suffix_and_splits_version() {
        assert_eq!(parse_install_spec("foo"), ("foo".into(), None));
        assert_eq!(parse_install_spec("foo@"), ("foo".into(), None));
        assert_eq!(
            parse_install_spec("foo.pairee@1.2"),
            ("foo".into(), Some("1.2"))
        );
    }

    #[test]
    fn install_spec_names_pass_the_install_guard() {
        for spec in ["demo-plugin", "demo-plugin.pairee", "demo_2@0.3.0"] {
            let (name, _) = parse_install_spec(spec);
            assert!(validate_identifier("name", &name).is_ok(), "{spec}");
        }
    }
}
