//! `pairee developer <cmd>`: the terminal front-end of the plugin
//! developer tools. Prompts and output live here; `plugin::developer_tool`
//! only returns results (it also runs inside the TUI, where printing would
//! corrupt the screen).

use super::help::{self, Topic};
use crate::config::localization::t;
use crate::plugin::developer_tool;
use anyhow::Result;

/// `pairee developer <cmd> [rest...]`.
pub async fn developer_command(cmd: &str, rest: &[String]) -> Result<()> {
    match cmd {
        "init" => match rest.first() {
            Some(name) => {
                let desc = prompt_line(&t("plugin_dev_prompt_description"))?;
                let author = prompt_line(&t("plugin_dev_prompt_author"))?;
                println!("{}", developer_tool::init(name, &desc, &author)?);
            }
            None => print!("{}", help::missing_argument(Topic::Developer, "init")),
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
        _ => print!("{}", help::unknown_command(Topic::Developer, cmd)),
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

/// What a key press does to a secret being typed.
#[derive(Debug, PartialEq, Eq)]
enum SecretKey {
    More,
    Done,
    Abort,
}

/// Applies `key` to the secret typed so far in `buf`.
fn apply_secret_key(buf: &mut String, key: crossterm::event::KeyEvent) -> SecretKey {
    use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
    if key.kind == KeyEventKind::Release {
        return SecretKey::More;
    }
    match key.code {
        KeyCode::Enter => SecretKey::Done,
        KeyCode::Esc => SecretKey::Abort,
        KeyCode::Char('c' | 'd') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            SecretKey::Abort
        }
        KeyCode::Backspace => {
            buf.pop();
            SecretKey::More
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            buf.push(c);
            SecretKey::More
        }
        _ => SecretKey::More,
    }
}

/// Prints `prompt` and reads one trimmed line **without echoing it** (a
/// token or password). When stdin is not a terminal (a pipe) the line is
/// read as it is.
pub fn prompt_secret(prompt: &str) -> Result<String> {
    use crossterm::event::{Event, read};
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        return prompt_line(prompt);
    }
    println!("{prompt}");
    /// Restores the terminal even if reading fails.
    struct RawMode;
    impl Drop for RawMode {
        fn drop(&mut self) {
            let _ = crossterm::terminal::disable_raw_mode();
        }
    }
    crossterm::terminal::enable_raw_mode()?;
    let _raw = RawMode;
    let mut secret = String::new();
    loop {
        if let Event::Key(key) = read()? {
            match apply_secret_key(&mut secret, key) {
                SecretKey::More => {}
                SecretKey::Done => break,
                SecretKey::Abort => {
                    drop(_raw);
                    println!();
                    anyhow::bail!(t("walk_error_cancelled"));
                }
            }
        }
    }
    drop(_raw);
    println!();
    Ok(secret.trim().to_string())
}

/// Asks for the license name of a plugin that has a LICENSE file but no
/// license in its manifest (empty when stdin is not a terminal).
fn ask_license() -> String {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        return String::new();
    }
    prompt_line(&t("plugin_dev_prompt_license")).unwrap_or_default()
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
        anyhow::bail!(t("plugin_dev_commit_empty"));
    }
    developer_tool::commit_registry_changes_with_progress(&commit_msg, None)?;
    println!("{}", t("plugin_dev_local_staged"));

    // 3. Optional token: open the PR automatically. It is typed without
    // echo, so it does not stay on screen or in the terminal's scrollback.
    let token = prompt_secret(&t("plugin_dev_submit_prompt"))?;
    if token.is_empty() {
        let temp_dir = crate::plugin::developer_tool::temp_registry_dir();
        let msg = t("plugin_dev_no_token_inst").replace("{}", &temp_dir.display().to_string());
        println!("\n{msg}");
    } else {
        println!("{}", t("plugin_dev_submit_sending"));
        match developer_tool::run_automatic_submit(&token, &commit_msg, &plugin_name).await {
            Ok(success_msg) => println!("{success_msg}"),
            Err(e) => anyhow::bail!(t("plugin_dev_submit_failed").replace("{}", &format!("{e:#}"))),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn press(buf: &mut String, code: KeyCode, mods: KeyModifiers) -> SecretKey {
        apply_secret_key(buf, KeyEvent::new(code, mods))
    }

    #[test]
    fn a_secret_is_typed_edited_and_submitted_without_being_shown() {
        let mut secret = String::new();
        for c in "ghp_ab".chars() {
            assert_eq!(
                press(&mut secret, KeyCode::Char(c), KeyModifiers::NONE),
                SecretKey::More
            );
        }
        press(&mut secret, KeyCode::Backspace, KeyModifiers::NONE);
        press(&mut secret, KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(secret, "ghp_ac");
        assert_eq!(
            press(&mut secret, KeyCode::Enter, KeyModifiers::NONE),
            SecretKey::Done
        );
        assert_eq!(
            press(&mut secret, KeyCode::Char('c'), KeyModifiers::CONTROL),
            SecretKey::Abort
        );
        assert_eq!(
            press(&mut secret, KeyCode::Esc, KeyModifiers::NONE),
            SecretKey::Abort
        );
        assert_eq!(secret, "ghp_ac", "control keys are not part of the secret");
    }
}
