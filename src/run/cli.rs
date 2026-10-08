//! Non-interactive subcommands: `--elevated-helper`, `pairee plugin ...` and
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
        Some("plugin") => {
            match args.get(2) {
                Some(cmd) => plugin_command(cmd, args.get(3..).unwrap_or_default()).await?,
                None => println!(
                    "Plugin CLI usage: pairee plugin [list|search|info|install|add|remove|pin|unpin|verify|check-updates|update]"
                ),
            }
            Ok(true)
        }
        Some("developer") => {
            match args.get(2) {
                Some(cmd) => developer_command(cmd, args.get(3..).unwrap_or_default()).await?,
                None => {
                    println!("Developer CLI usage: pairee developer [init|lint|package|submit]")
                }
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// `pairee plugin <cmd> [rest...]`.
async fn plugin_command(cmd: &str, rest: &[String]) -> Result<()> {
    let first = rest.first().map(String::as_str);
    match (cmd, first) {
        ("list", _) => plugin::updater::list_installed().await?,
        ("search", Some(query)) => plugin::updater::search(query).await?,
        ("search", None) => println!("Error: search requires a query string"),
        ("info", Some(name)) => plugin::updater::show_info(name).await?,
        ("info", None) => println!("Error: info requires a plugin name"),
        ("install" | "add", Some(_)) => install_plugins(rest).await,
        ("install" | "add", None) => {
            println!("Error: install command requires at least one plugin name")
        }
        ("remove", Some(name)) => plugin::updater::remove(name)?,
        ("remove", None) => println!("Error: remove requires a plugin name"),
        ("pin", Some(name)) => plugin::updater::pin(name, true)?,
        ("pin", None) => println!("Error: pin requires a plugin name"),
        ("unpin", Some(name)) => plugin::updater::pin(name, false)?,
        ("unpin", None) => println!("Error: unpin requires a plugin name"),
        ("verify", _) => plugin::updater::verify().await?,
        ("check-updates", _) => plugin::updater::check_updates().await?,
        ("update", name) => plugin::updater::update(name).await?,
        _ => println!(
            "Unknown plugin command. Available: list, search, info, install, add, remove, pin, unpin, verify, check-updates, update"
        ),
    }
    Ok(())
}

/// Installs each `name[@version]`; a failure is reported and the rest continue.
async fn install_plugins(specs: &[String]) {
    for spec in specs {
        let (name, version) = parse_install_spec(spec);
        if let Err(e) = plugin::updater::install(&name, version).await {
            println!("Error installing plugin '{}': {:?}", name, e);
        }
    }
}

/// `foo@1.2` → (`foo.pairee`, `Some("1.2")`); the `.pairee` suffix is added when missing.
fn parse_install_spec(spec: &str) -> (String, Option<&str>) {
    let mut parts = spec.split('@');
    let base = parts.next().unwrap_or_default();
    let version = parts.next();
    let name = if base.ends_with(".pairee") {
        base.to_string()
    } else {
        format!("{}.pairee", base)
    };
    (name, version)
}

/// `pairee developer <cmd> [rest...]`.
async fn developer_command(cmd: &str, rest: &[String]) -> Result<()> {
    match cmd {
        "init" => match rest.first() {
            Some(name) => {
                let desc = prompt_line("Enter plugin description:")?;
                let author = prompt_line("Enter plugin author:")?;
                plugin::developer_tool::init(name, &desc, &author, true)?;
            }
            None => println!("Error: init requires a plugin name"),
        },
        "lint" => plugin::developer_tool::lint()?,
        "package" => plugin::developer_tool::package()?,
        "submit" => plugin::developer_tool::submit().await?,
        _ => println!("Unknown developer command. Available: init, lint, package, submit"),
    }
    Ok(())
}

/// Prints `prompt` and reads one trimmed line from stdin.
fn prompt_line(prompt: &str) -> Result<String> {
    println!("{prompt}");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_install_spec;

    #[test]
    fn install_spec_adds_suffix_and_splits_version() {
        assert_eq!(parse_install_spec("foo"), ("foo.pairee".into(), None));
        assert_eq!(
            parse_install_spec("foo.pairee@1.2"),
            ("foo.pairee".into(), Some("1.2"))
        );
    }
}
