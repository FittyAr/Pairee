use super::super::progress::begin_dev_op;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PluginMenuState};
use crate::config::localization::t;
use crate::plugin::developer_tool;
use std::path::PathBuf;

pub fn handle_wizard_enter(
    state: &mut AppState,
    context: &mut AppContext,
    menu: &mut PluginMenuState,
) {
    if menu.dev_wizard_step == 1 {
        let name = menu.search_query.clone().trim().to_string();
        if !name.is_empty() {
            menu.dev_wizard_data.push(name);
            menu.search_query.clear();
            menu.dev_wizard_step = 2;
        }
    } else if menu.dev_wizard_step == 2 {
        let desc = menu.search_query.clone().trim().to_string();
        menu.dev_wizard_data.push(desc);
        menu.search_query.clear();
        menu.dev_wizard_step = 3;
    } else if menu.dev_wizard_step == 3 {
        let author = menu.search_query.clone().trim().to_string();
        menu.dev_wizard_data.push(author);
        menu.search_query.clear();
        menu.editing_query = false;
        menu.dev_wizard_step = 0;

        let name = menu.dev_wizard_data[0].clone();
        let desc = menu.dev_wizard_data[1].clone();
        let author = menu.dev_wizard_data[2].clone();
        menu.dev_wizard_data.clear();
        let plugins_dev_dir =
            std::path::PathBuf::from(context.config.settings.plugins_dev_dir.clone());
        let folder_name = if name.ends_with(".pairee") {
            name.clone()
        } else {
            format!("{}.pairee", name)
        };
        let target_path = PathBuf::from(&plugins_dev_dir).join(&folder_name);

        let _ = std::fs::create_dir_all(&target_path);
        menu.dev_results = format!(
            "{} '{}'…",
            t("plugin_dev_progress_initializing"),
            folder_name
        );

        let tx = begin_dev_op(state, t("plugin_dev_progress_creating_dir"));
        let parent_for_task = plugins_dev_dir.clone();
        tokio::task::spawn_blocking(move || {
            let res = developer_tool::init_with_progress_in(
                &folder_name,
                &desc,
                &author,
                false,
                Some(tx.clone()),
                &parent_for_task,
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
                    let result_text =
                        format!("{}\n\n{}", result_text, t("plugin_dev_init_select_hint"));
                    developer_tool::progress_finish(Some(tx), Some(result_text), None);
                }
                Err(e) => {
                    let err = t("plugin_dev_init_err").replace("{:?}", &format!("{}", e));
                    developer_tool::progress_finish(Some(tx), None, Some(err));
                }
            }
        });
        menu.installed = super::super::reload_installed_plugins(context, &None);
    } else if menu.dev_wizard_step == 5 {
        let commit_msg = menu.search_query.clone().trim().to_string();
        if !commit_msg.is_empty() {
            menu.dev_wizard_data.push(commit_msg);
            menu.search_query.clear();
            menu.dev_wizard_step = 6;
        }
    } else if menu.dev_wizard_step == 6 {
        let token = menu.search_query.clone().trim().to_string();
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
        let commit_msg_for_blocking = commit_msg.clone();
        let plugin_path_for_blocking = plugin_path.clone();

        tokio::task::spawn_blocking(move || {
            let mut local_err: Option<String> = None;
            match developer_tool::package_to_registry_with_progress(
                &plugin_path_for_blocking,
                Some(tx.clone()),
            ) {
                Ok(_) => {
                    if let Err(e) = developer_tool::commit_registry_changes_with_progress(
                        &commit_msg_for_blocking,
                        Some(tx.clone()),
                    ) {
                        local_err = Some(
                            t("plugin_dev_err_git_commit").replace("{:?}", &format!("{:?}", e)),
                        );
                    }
                }
                Err(e) => {
                    local_err = Some(
                        t("plugin_dev_err_package_registry").replace("{:?}", &format!("{:?}", e)),
                    );
                }
            }

            if let Some(err) = local_err {
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

            let tx_for_async = tx.clone();
            let commit_msg_async = commit_msg.clone();
            let manifest_path = plugin_path.join("manifest.toml");
            let mut plugin_name = String::new();
            if let Ok(manifest_content) = std::fs::read_to_string(&manifest_path)
                && let Ok(manifest) =
                    crate::plugin::loader::PluginManifest::parse(&manifest_content)
            {
                plugin_name = manifest.name;
            }

            tokio::spawn(async move {
                use crate::app::input_popup::plugin_menu::toast::{Toast, notify};
                match developer_tool::run_automatic_submit(&token, &commit_msg_async, &plugin_name)
                    .await
                {
                    Ok(msg) => {
                        let title = t("plugin_dev_toast_submitted_title");
                        notify(Toast { title, msg }, "info").await;
                        developer_tool::progress_finish(
                            Some(tx_for_async),
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
                        developer_tool::progress_finish(Some(tx_for_async), None, Some(msg));
                    }
                }
            });
        });
        menu.installed = super::super::reload_installed_plugins(context, &None);
    }
}
