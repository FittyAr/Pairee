#![allow(clippy::all)]
use anyhow::{Context, Result};
use simplelog::*;
use std::env;
use std::path::PathBuf;

mod app;
mod config;
mod fs;
mod git;
mod keybindings;
mod plugin;
mod terminal;
mod ui;
mod update;

#[tokio::main]
async fn main() -> Result<()> {
    // Install the rustls ring crypto provider for reqwest on non-Windows platforms
    #[cfg(not(target_os = "windows"))]
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Intercept elevated helper requests
    let args: Vec<String> = env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--elevated-helper") {
        if pos + 1 < args.len() {
            let temp_file = PathBuf::from(&args[pos + 1]);
            fs::elevated_helper::run_elevated_helper_loop(&temp_file)?;
        } else {
            anyhow::bail!("Missing temp file argument for --elevated-helper");
        }
        return Ok(());
    }

    // Intercept the privileged "list a directory" subcommand. This is
    // used by `read_directory_as_admin` on Linux so that the elevation
    // runs the same binary (no external python3 dependency) and the
    // output is JSON, parsed by the caller.
    if let Some(pos) = args.iter().position(|a| a == "--list-dir-elevated") {
        if pos + 1 < args.len() {
            let dir = PathBuf::from(&args[pos + 1]);
            let entries = fs::list::list_dir_elevated(&dir)?;
            // Emit a compact JSON array on stdout. The caller parses
            // this with serde_json.
            println!("{}", serde_json::to_string(&entries)?);
        } else {
            anyhow::bail!("Missing path argument for --list-dir-elevated");
        }
        return Ok(());
    }

    // Intercept plugin and developer subcommands
    if args.len() > 1 {
        if args[1] == "plugin" {
            if args.len() > 2 {
                match args[2].as_str() {
                    "list" => {
                        let rows = plugin::updater::list_installed().await?;
                        if rows.is_empty() {
                            println!("Installed Plugins:");
                            println!("  (none)");
                            return Ok(());
                        }
                        println!("Installed Plugins:");
                        for r in &rows {
                            let pin = if r.pinned { " [PINNED]" } else { "" };
                            let trust = if r.trusted {
                                " [TRUSTED]"
                            } else {
                                " [UNTRUSTED]"
                            };
                            let blocked = match &r.blocked {
                                Some(_) => " [BLOCKED]".to_string(),
                                None => String::new(),
                            };
                            let update = match &r.update_available {
                                Some(v) => format!(" (Update available: v{})", v),
                                None => String::new(),
                            };
                            println!(
                                "  - {} v{}{}{}{}{}",
                                r.name, r.version, pin, trust, blocked, update
                            );
                        }
                        return Ok(());
                    }
                    "search" => {
                        if args.len() > 3 {
                            let matches = plugin::updater::search(&args[3]).await?;
                            println!("Search results for '{}':", args[3]);
                            for m in &matches {
                                let langs = if m.languages.is_empty() {
                                    String::new()
                                } else {
                                    format!(
                                        " [{}]",
                                        m.languages
                                            .iter()
                                            .map(|l| l.to_uppercase())
                                            .collect::<Vec<_>>()
                                            .join(" ")
                                    )
                                };
                                let hook = if m.is_hook { " [Hook]" } else { "" };
                                println!(
                                    "* {} v{} by {}{}{}",
                                    m.name, m.version, m.author, hook, langs
                                );
                                if let Some(d) = &m.description {
                                    println!("  Description: {}", d);
                                }
                                println!();
                            }
                        } else {
                            println!("Error: search requires a query string");
                        }
                        return Ok(());
                    }
                    "info" => {
                        if args.len() > 3 {
                            match plugin::updater::show_info(&args[3]).await {
                                Ok(info) => {
                                    println!("Plugin: {}", info.name);
                                    println!("Version: {}", info.version);
                                    println!("Author: {}", info.author);
                                    if let Some(d) = &info.description {
                                        println!("Description: {}", d);
                                    }
                                    if let Some(m) = &info.min_pairee {
                                        println!("Requires Pairee: >= {}", m);
                                    }
                                    if !info.languages.is_empty() {
                                        println!(
                                            "Supported languages: {}",
                                            info.languages.join(", ")
                                        );
                                    }
                                    if !info.hooks.is_empty() {
                                        println!("Subscribes to hooks: {}", info.hooks.join(", "));
                                    }
                                    if !info.files.is_empty() {
                                        println!("Files:");
                                        for f in &info.files {
                                            println!("  - {}", f);
                                        }
                                    }
                                }
                                Err(e) => println!("Error: {:?}", e),
                            }
                        } else {
                            println!("Error: info requires a plugin name");
                        }
                        return Ok(());
                    }
                    "install" | "add" => {
                        if args.len() > 3 {
                            for part in &args[3..] {
                                let (name, version) = if part.contains('@') {
                                    let split: Vec<&str> = part.split('@').collect();
                                    let clean_name = if split[0].ends_with(".pairee") {
                                        split[0].to_string()
                                    } else {
                                        format!("{}.pairee", split[0])
                                    };
                                    (clean_name, Some(split[1]))
                                } else {
                                    let clean_name = if part.ends_with(".pairee") {
                                        part.to_string()
                                    } else {
                                        format!("{}.pairee", part)
                                    };
                                    (clean_name, None)
                                };
                                match plugin::updater::install(&name, version).await {
                                    Ok(()) => println!("Installed plugin '{}'", name),
                                    Err(e) => {
                                        println!("Error installing plugin '{}': {:?}", name, e)
                                    }
                                }
                            }
                        } else {
                            println!("Error: install command requires at least one plugin name");
                        }
                        return Ok(());
                    }
                    "remove" => {
                        if args.len() > 3 {
                            match plugin::updater::remove(&args[3]) {
                                Ok(()) => println!("Removed plugin '{}'", args[3]),
                                Err(e) => println!("Error: {:?}", e),
                            }
                        } else {
                            println!("Error: remove requires a plugin name");
                        }
                        return Ok(());
                    }
                    "pin" => {
                        if args.len() > 3 {
                            match plugin::updater::pin(&args[3], true) {
                                Ok(p) => {
                                    println!("Set pin status of plugin '{}' to {}.", args[3], p)
                                }
                                Err(e) => println!("Error: {:?}", e),
                            }
                        } else {
                            println!("Error: pin requires a plugin name");
                        }
                        return Ok(());
                    }
                    "unpin" => {
                        if args.len() > 3 {
                            match plugin::updater::pin(&args[3], false) {
                                Ok(p) => {
                                    println!("Set pin status of plugin '{}' to {}.", args[3], p)
                                }
                                Err(e) => println!("Error: {:?}", e),
                            }
                        } else {
                            println!("Error: unpin requires a plugin name");
                        }
                        return Ok(());
                    }
                    "verify" => {
                        let report = plugin::updater::verify().await?;
                        for entry in &report.entries {
                            println!("Plugin: {} v{}", entry.name, entry.version);
                            for (file, status) in &entry.files {
                                let status_str = match status {
                                    plugin::updater::VerifyEntryStatus::Ok => "OK".to_string(),
                                    plugin::updater::VerifyEntryStatus::MissingFile => {
                                        "MISSING".to_string()
                                    }
                                    plugin::updater::VerifyEntryStatus::Blocked(r) => {
                                        format!("BLOCKED ({})", r)
                                    }
                                    plugin::updater::VerifyEntryStatus::HashMismatch {
                                        expected,
                                        actual,
                                    } => {
                                        format!(
                                            "HASH MISMATCH (expected {}, got {})",
                                            expected, actual
                                        )
                                    }
                                    plugin::updater::VerifyEntryStatus::HashError(e) => {
                                        format!("HASH ERROR ({})", e)
                                    }
                                };
                                println!("  - {}: {}", file, status_str);
                            }
                        }
                        if report.clean {
                            println!("All plugins verified successfully (integrity clean).");
                        } else {
                            println!("Integrity verification failed for one or more plugins.");
                        }
                        return Ok(());
                    }
                    "check-updates" => {
                        let updates = plugin::updater::check_updates().await?;
                        if updates.is_empty() {
                            println!("No plugins installed.");
                            return Ok(());
                        }
                        let mut count = 0;
                        for u in &updates {
                            match &u.status {
                                plugin::updater::UpdateStatus::UpToDate => {
                                    if let Some(latest) = &u.latest {
                                        println!("  - {}: v{} (up to date)", u.name, latest);
                                    }
                                }
                                plugin::updater::UpdateStatus::Pinned => {
                                    println!(
                                        "  - {}: {} -> {} [PINNED] (update skipped)",
                                        u.name,
                                        u.installed,
                                        u.latest.as_deref().unwrap_or("?")
                                    );
                                    count += 1;
                                }
                                plugin::updater::UpdateStatus::Blocked(r) => {
                                    println!(
                                        "  - {}: v{} [BLOCKED] Reason: {}",
                                        u.name, u.installed, r
                                    );
                                    count += 1;
                                }
                                plugin::updater::UpdateStatus::Updated { from, to } => {
                                    println!("  - {}: {} -> {}", u.name, from, to);
                                    count += 1;
                                }
                                plugin::updater::UpdateStatus::Failed(_) => {}
                            }
                        }
                        if count == 0 {
                            println!("All plugins are up to date.");
                        } else {
                            println!(
                                "Found {} plugin update(s). Run 'pairee plugin update' to update non-pinned plugins.",
                                count
                            );
                        }
                        return Ok(());
                    }
                    "update" => {
                        let name = if args.len() > 3 {
                            Some(args[3].as_str())
                        } else {
                            None
                        };
                        let report = plugin::updater::update(name).await?;
                        for (n, status) in &report.items {
                            match status {
                                plugin::updater::UpdateStatus::Updated { from, to } => {
                                    println!("Updated '{}': {} -> {}", n, from, to)
                                }
                                plugin::updater::UpdateStatus::UpToDate => {
                                    println!("'{}' is already up to date.", n)
                                }
                                plugin::updater::UpdateStatus::Pinned => {
                                    println!("Skipping pinned plugin '{}'.", n)
                                }
                                plugin::updater::UpdateStatus::Blocked(r) => {
                                    println!(
                                        "WARNING: plugin '{}' is BLOCKED: {}. Removing for safety.",
                                        n, r
                                    );
                                    let _ = plugin::updater::remove(n);
                                }
                                plugin::updater::UpdateStatus::Failed(e) => {
                                    println!("✗ Failed to update '{}': {}", n, e)
                                }
                            }
                        }
                        println!(
                            "Updated {} plugin(s); {} failed.",
                            report.updated_count(),
                            report.failed_count()
                        );
                        return Ok(());
                    }
                    _ => {
                        println!(
                            "Unknown plugin command. Available: list, search, info, install, add, remove, pin, unpin, verify, check-updates, update"
                        );
                    }
                }
            } else {
                println!(
                    "Plugin CLI usage: pairee plugin [list|search|info|install|add|remove|pin|unpin|verify|check-updates|update]"
                );
            }
            return Ok(());
        } else if args[1] == "developer" {
            if args.len() > 2 {
                match args[2].as_str() {
                    "init" => {
                        if args.len() > 3 {
                            let name = &args[3];
                            println!("Enter plugin description:");
                            let mut desc = String::new();
                            std::io::stdin().read_line(&mut desc)?;
                            let desc = desc.trim().to_string();

                            println!("Enter plugin author:");
                            let mut author = String::new();
                            std::io::stdin().read_line(&mut author)?;
                            let author = author.trim().to_string();

                            plugin::developer_tool::init(name, &desc, &author, true)?;
                        } else {
                            println!("Error: init requires a plugin name");
                        }
                        return Ok(());
                    }
                    "lint" => {
                        plugin::developer_tool::lint()?;
                        return Ok(());
                    }
                    "package" => {
                        plugin::developer_tool::package()?;
                        return Ok(());
                    }
                    "submit" => {
                        plugin::developer_tool::submit().await?;
                        return Ok(());
                    }
                    _ => {
                        println!(
                            "Unknown developer command. Available: init, lint, package, submit"
                        );
                    }
                }
            } else {
                println!("Developer CLI usage: pairee developer [init|lint|package|submit]");
            }
            return Ok(());
        }
    }

    // 0. Check if we need to spawn a standalone terminal window
    if terminal::standalone::check_and_launch_standalone().unwrap_or(false) {
        return Ok(());
    }

    // 1. Load configuration TOML profiles
    let config =
        config::AppConfig::load_or_create().context("Failed to initialize config files")?;

    // 2. Setup application debug logger
    let log_path = config::paths::get_log_file_path();
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(file) = std::fs::File::create(&log_path) {
        let _ = WriteLogger::init(LevelFilter::Debug, Config::default(), file);
    }

    log::info!("Starting Pairee application...");
    git::unused_keepalive();

    // 3. Resolve starting folders for panels
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let right_dir = current_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| current_dir.clone());

    // 4. Initialize context and state containers
    let context = app::AppContext::new(config);
    let mut state = app::AppState::new(current_dir, right_dir);
    state.case_sensitive_sort = context.config.settings.case_sensitive_sort;
    state.treat_digits_as_numbers = context.config.settings.treat_digits_as_numbers;
    state.sorting_collation = context.config.settings.sorting_collation.clone();
    state.req_admin_reading = context.config.settings.req_admin_reading;
    // Panel settings
    state.select_folders = context.config.settings.select_folders;
    state.sort_folder_names_by_extension = context.config.settings.sort_folder_names_by_extension;
    state.show_dotdot_in_root_folders = context.config.settings.show_dotdot_in_root_folders;
    state.disable_panel_update_object_count =
        context.config.settings.disable_panel_update_object_count;

    // 5. Launch background update check (if enabled)
    if context.config.settings.auto_update_check {
        let (tx, rx) = tokio::sync::oneshot::channel();
        update::checker::UpdateChecker::check_in_background(tx);
        state.update_check_rx = Some(rx);
        state.update_status = update::UpdateStatus::Checking;
    }

    // 5.5. Initialize and load plugins. The plugin request
    // channel is already created by `AppState::new` (see
    // `app::state::AppState::new` and
    // `plugin::manager::PLUGIN_REQ_TX`); we just kick off
    // the eager loader here.
    plugin::PluginManager::load_all_plugins(&context).await;

    // 6. Hand execution over to main loop
    app::run(context, state).await?;

    log::info!("Pairee exited cleanly.");
    Ok(())
}
