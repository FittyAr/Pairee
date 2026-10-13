//! Non-interactive subcommands: `--elevated-helper`, `--version`, `pairee plugin ...` and
//! `pairee developer ...`. Each runs to completion without starting the TUI.

use anyhow::Result;
use std::path::PathBuf;

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
    match args.get(1).map(String::as_str) {
        Some("--version" | "-V") => {
            println!("pairee {}", env!("CARGO_PKG_VERSION"));
            Ok(true)
        }
        Some("plugin") => {
            match args.get(2) {
                Some(cmd) => plugin_command(cmd, args.get(3..).unwrap_or_default()).await?,
                None => println!(
                    "Plugin CLI usage: pairee plugin [list|search|info|install|add|remove|pin|unpin|verify|check-updates|update]"
                ),
            }
            Ok(true)
        }
        Some("keymap") => {
            keymap_command(args.get(2..).unwrap_or_default());
            Ok(true)
        }
        Some("developer") => {
            match args.get(2) {
                Some(cmd) => {
                    super::dev_cli::developer_command(cmd, args.get(3..).unwrap_or_default())
                        .await?
                }
                None => {
                    println!("Developer CLI usage: pairee developer [init|lint|package|submit]")
                }
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// `pairee keymap print [--preset <name>] [--lang <code>]`: the Markdown
/// reference of a shipped preset (the source of `help/<lang>/keymap_*.md`).
fn keymap_command(rest: &[String]) {
    use crate::keybindings::embedded::{PRESETS, normalize_preset_name, preset_toml};
    let value = |flag: &str| {
        rest.iter()
            .position(|a| a == flag)
            .and_then(|i| rest.get(i + 1))
            .cloned()
    };
    if rest.first().map(String::as_str) != Some("print") {
        println!("Keymap CLI usage: pairee keymap print [--preset <name>] [--lang <code>]");
        return;
    }
    let preset = normalize_preset_name(&value("--preset").unwrap_or_else(|| "norton".into()));
    if preset_toml(&preset).is_none() || !PRESETS.iter().any(|(n, _)| *n == preset) {
        let names: Vec<&str> = PRESETS.iter().map(|(n, _)| *n).collect();
        println!(
            "Unknown preset '{preset}'. Built-in presets: {}",
            names.join(", ")
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
        ("search", None) => usage("Error: search requires a query string"),
        ("info", None) => usage("Error: info requires a plugin name"),
        ("install" | "add", None) => {
            usage("Error: install command requires at least one plugin name")
        }
        ("remove", None) => usage("Error: remove requires a plugin name"),
        ("pin", None) => usage("Error: pin requires a plugin name"),
        ("unpin", None) => usage("Error: unpin requires a plugin name"),
        _ => usage(
            "Unknown plugin command. Available: list, search, info, install, add, remove, pin, unpin, verify, check-updates, update",
        ),
    };
    println!("{report}");
    Ok(())
}

/// A report holding one usage message.
fn usage(text: &str) -> plugin::updater::Report {
    let mut report = plugin::updater::Report::default();
    report.lines.push(text.to_string());
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
