use super::super::progress::begin_dev_op;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PluginMenuState};
use crate::config::localization::t;
use crate::plugin::developer_tool;
use std::path::{Path, PathBuf};

/// Progress channel of a developer operation.
type DevTx = tokio::sync::mpsc::UnboundedSender<crate::app::state::DevProgress>;

pub fn handle_wizard_enter(
    state: &mut AppState,
    context: &mut AppContext,
    menu: &mut PluginMenuState,
) {
    match menu.dev_wizard_step {
        // Plugin name (required) → description → author.
        1 => store_answer(menu, true, 2),
        2 => store_answer(menu, false, 3),
        3 => {
            store_answer(menu, false, 0);
            menu.editing_query = false;
            finish_init_wizard(state, context, menu);
        }
        // Commit message (required) → token.
        5 => store_answer(menu, true, 6),
        6 => finish_submit_wizard(state, context, menu),
        _ => {}
    }
}

/// Stores the trimmed query as the next wizard answer and moves to `next`
/// (an empty `required` answer keeps the step).
fn store_answer(menu: &mut PluginMenuState, required: bool, next: usize) {
    let answer = menu.search_query.trim().to_string();
    if required && answer.is_empty() {
        return;
    }
    menu.dev_wizard_data.push(answer);
    menu.search_query.clear();
    menu.dev_wizard_step = next;
}

/// Creates the plugin skeleton from the name, description and author answers.
fn finish_init_wizard(state: &mut AppState, context: &mut AppContext, menu: &mut PluginMenuState) {
    let name = menu.dev_wizard_data[0].clone();
    let desc = menu.dev_wizard_data[1].clone();
    let author = menu.dev_wizard_data[2].clone();
    menu.dev_wizard_data.clear();
    let plugins_dev_dir = PathBuf::from(context.config.settings.plugins_dev_dir.clone());
    let folder_name = if name.ends_with(".pairee") {
        name
    } else {
        format!("{}.pairee", name)
    };
    let target_path = plugins_dev_dir.join(&folder_name);

    let _ = std::fs::create_dir_all(&target_path);
    menu.dev_results = format!(
        "{} '{}'…",
        t("plugin_dev_progress_initializing"),
        folder_name
    );

    let tx = begin_dev_op(state, t("plugin_dev_progress_creating_dir"));
    tokio::task::spawn_blocking(move || {
        let res = developer_tool::init_with_progress_in(
            &folder_name,
            &desc,
            &author,
            false,
            Some(tx.clone()),
            &plugins_dev_dir,
        );
        match res {
            Ok(_) => {
                let name_without_suffix = folder_name
                    .strip_suffix(".pairee")
                    .unwrap_or(&folder_name)
                    .to_string();
                let result_text = t("plugin_dev_init_ok")
                    .replace("{}", &name_without_suffix)
                    .replace("{:?}", &target_path.to_string_lossy());
                let result_text = format!(
                    "{}

{}",
                    result_text,
                    t("plugin_dev_init_select_hint")
                );
                developer_tool::progress_finish(Some(tx), Some(result_text), None);
            }
            Err(e) => {
                let err = t("plugin_dev_init_err").replace("{:?}", &format!("{}", e));
                developer_tool::progress_finish(Some(tx), None, Some(err));
            }
        }
    });
    menu.installed = super::super::reload_installed_plugins(context, &None);
}

/// Packages the plugin into the registry, commits, and submits with the token answer.
fn finish_submit_wizard(
    state: &mut AppState,
    context: &mut AppContext,
    menu: &mut PluginMenuState,
) {
    let token = menu.search_query.trim().to_string();
    let plugin_path_str = menu.dev_wizard_data[0].clone();
    let commit_msg = menu.dev_wizard_data[1].clone();
    menu.dev_wizard_data.clear();
    menu.editing_query = false;
    menu.dev_wizard_step = 0;
    menu.search_query.clear();

    menu.dev_results = format!(
        "{} '{}'…",
        t("plugin_dev_progress_submitting"),
        plugin_path_str
    );

    let tx = begin_dev_op(state, t("plugin_dev_progress_packaging"));
    let plugin_path = PathBuf::from(&plugin_path_str);
    tokio::task::spawn_blocking(move || {
        if let Err(err) = package_and_commit(&plugin_path, &commit_msg, &tx) {
            developer_tool::progress_finish(Some(tx), None, Some(err));
            return;
        }
        if token.is_empty() {
            let temp_dir = crate::config::paths::get_cache_dir().join("temp_registry");
            let result =
                t("plugin_dev_no_token_inst").replace("{}", &temp_dir.display().to_string());
            developer_tool::progress_finish(Some(tx), Some(result), None);
            return;
        }
        let plugin_name = manifest_name(&plugin_path);
        tokio::spawn(submit_in_background(token, commit_msg, plugin_name, tx));
    });
    menu.installed = super::super::reload_installed_plugins(context, &None);
}

/// Packages `plugin_path` into the local registry and commits it; the error is localized.
fn package_and_commit(plugin_path: &Path, commit_msg: &str, tx: &DevTx) -> Result<(), String> {
    developer_tool::package_to_registry_with_progress(plugin_path, Some(tx.clone()))
        .map_err(|e| t("plugin_dev_err_package_registry").replace("{:?}", &format!("{:?}", e)))?;
    developer_tool::commit_registry_changes_with_progress(commit_msg, Some(tx.clone()))
        .map_err(|e| t("plugin_dev_err_git_commit").replace("{:?}", &format!("{:?}", e)))
}

/// Plugin name from `manifest.toml`, empty when it cannot be read.
fn manifest_name(plugin_path: &Path) -> String {
    std::fs::read_to_string(plugin_path.join("manifest.toml"))
        .ok()
        .and_then(|content| crate::plugin::loader::PluginManifest::parse(&content).ok())
        .map(|manifest| manifest.name)
        .unwrap_or_default()
}

/// Forks, pushes and opens the registry pull request, reporting with a toast.
async fn submit_in_background(token: String, commit_msg: String, plugin_name: String, tx: DevTx) {
    use crate::app::input_popup::plugin_menu::toast::{Toast, notify};
    match developer_tool::run_automatic_submit(&token, &commit_msg, &plugin_name).await {
        Ok(msg) => {
            let title = t("plugin_dev_toast_submitted_title");
            notify(Toast { title, msg }, "info").await;
            developer_tool::progress_finish(
                Some(tx),
                Some(t("plugin_dev_fork_push_bg").to_string()),
                None,
            );
        }
        Err(e) => {
            let title = t("plugin_dev_toast_submit_fail_title");
            let msg = format!("{:?}", e);
            notify(
                Toast {
                    title,
                    msg: msg.clone(),
                },
                "error",
            )
            .await;
            developer_tool::progress_finish(Some(tx), None, Some(msg));
        }
    }
}
