//! `pairee developer <cmd>`: the terminal front-end of the plugin
//! developer tools. Prompts and output live here; `plugin::developer_tool`
//! only returns results (it also runs inside the TUI, where printing would
//! corrupt the screen).

use crate::config::localization::t;
use crate::plugin::developer_tool;
use anyhow::Result;

/// `pairee developer <cmd> [rest...]`.
pub async fn developer_command(cmd: &str, rest: &[String]) -> Result<()> {
    match cmd {
        "init" => match rest.first() {
            Some(name) => {
                let desc = prompt_line("Enter plugin description:")?;
                let author = prompt_line("Enter plugin author:")?;
                println!("{}", developer_tool::init(name, &desc, &author)?);
            }
            None => println!("Error: init requires a plugin name"),
        },
        "lint" => {
            let path = std::env::current_dir()?;
            println!("{}", developer_tool::lint_with_progress(&path, None)?);
        }
        "package" => {
            let path = std::env::current_dir()?;
            println!(
                "{}",
                developer_tool::package_to_registry(&path, Some(&ask_license))?
            );
        }
        "submit" => submit().await?,
        _ => println!("Unknown developer command. Available: init, lint, package, submit"),
    }
    Ok(())
}

/// Prints `prompt` and reads one trimmed line from stdin.
pub fn prompt_line(prompt: &str) -> Result<String> {
    println!("{prompt}");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

/// Asks for the license name of a plugin that has a LICENSE file but no
/// license in its manifest (empty when stdin is not a terminal).
fn ask_license() -> String {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        return String::new();
    }
    prompt_line(
        "LICENSE file detected, but no license name specified in manifest.toml.\n\
         Please enter the license name (e.g. MIT, GPL-3.0, Apache-2.0):",
    )
    .unwrap_or_default()
}

/// Interactive submit wizard: package, commit locally, then open a PR when
/// a GitHub token is given.
async fn submit() -> Result<()> {
    println!("{}", t("plugin_dev_submit_wizard"));

    let path = std::env::current_dir()?;
    if let Err(err_msg) = developer_tool::validate_for_publish(&path) {
        anyhow::bail!(err_msg);
    }
    let content = std::fs::read_to_string(path.join("manifest.toml"))?;
    let plugin_name = crate::plugin::loader::PluginManifest::parse(&content)?.name;

    // 1. Package into the temp registry clone.
    println!(
        "{}",
        developer_tool::package_to_registry(&path, Some(&ask_license))?
    );

    // 2. Commit locally.
    let commit_msg = prompt_line(&t("plugin_enter_commit_desc"))?;
    if commit_msg.is_empty() {
        anyhow::bail!("Commit message cannot be empty.");
    }
    developer_tool::commit_registry_changes_with_progress(&commit_msg, None)?;
    println!("{}", t("plugin_dev_local_staged"));

    // 3. Optional token: open the PR automatically.
    let token = prompt_line(&t("plugin_dev_submit_prompt"))?;
    if token.is_empty() {
        let temp_dir = crate::config::paths::get_cache_dir().join("temp_registry");
        let msg = t("plugin_dev_no_token_inst").replace("{}", &temp_dir.display().to_string());
        println!("\n{msg}");
    } else {
        println!("{}", t("plugin_dev_submit_sending"));
        match developer_tool::run_automatic_submit(&token, &commit_msg, &plugin_name).await {
            Ok(success_msg) => println!("{success_msg}"),
            Err(e) => anyhow::bail!("Automatic submission failed: {:?}", e),
        }
    }
    Ok(())
}
