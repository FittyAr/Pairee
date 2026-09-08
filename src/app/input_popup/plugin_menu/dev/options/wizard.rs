use super::super::progress::begin_dev_op;
use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::config::localization::t;
use crate::plugin::developer_tool;
use std::path::PathBuf;

#[allow(clippy::too_many_arguments)]
pub fn handle_wizard_enter(
    state: &mut AppState,
    context: &mut AppContext,
    installed: &mut Vec<(String, String, bool, bool, Option<String>)>,
    search_query: &mut String,
    editing_query: &mut bool,
    dev_results: &mut String,
    dev_wizard_step: &mut usize,
    dev_wizard_data: &mut Vec<String>,
) {
    if *dev_wizard_step == 1 {
        let name = search_query.clone().trim().to_string();
        if !name.is_empty() {
            dev_wizard_data.push(name);
            search_query.clear();
            *dev_wizard_step = 2;
        }
    } else if *dev_wizard_step == 2 {
        let desc = search_query.clone().trim().to_string();
        dev_wizard_data.push(desc);
        search_query.clear();
        *dev_wizard_step = 3;
    } else if *dev_wizard_step == 3 {
        let author = search_query.clone().trim().to_string();
        dev_wizard_data.push(author);
        search_query.clear();
        *editing_query = false;
        *dev_wizard_step = 0;

        let name = dev_wizard_data[0].clone();
        let desc = dev_wizard_data[1].clone();
        let author = dev_wizard_data[2].clone();
        dev_wizard_data.clear();
        let plugins_dev_dir =
            std::path::PathBuf::from(context.config.settings.plugins_dev_dir.clone());
        let folder_name = if name.ends_with(".pairee") {
            name.clone()
        } else {
            format!("{}.pairee", name)
        };
        let target_path = PathBuf::from(&plugins_dev_dir).join(&folder_name);

        let _ = std::fs::create_dir_all(&target_path);
        *dev_results = format!(
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
        *installed = super::super::reload_installed_plugins(context, &None);
    } else if *dev_wizard_step == 5 {
        let commit_msg = search_query.clone().trim().to_string();
        if !commit_msg.is_empty() {
            dev_wizard_data.push(commit_msg);
            search_query.clear();
            *dev_wizard_step = 6;
        }
    } else if *dev_wizard_step == 6 {
        let token = search_query.clone().trim().to_string();
        let plugin_path_str = dev_wizard_data[0].clone();
        let commit_msg = dev_wizard_data[1].clone();
        dev_wizard_data.clear();
        *editing_query = false;
        *dev_wizard_step = 0;
        search_query.clear();

        *dev_results = format!(
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
                let notify_tx = crate::plugin::PluginManager::get_sender();
                match developer_tool::run_automatic_submit(&token, &commit_msg_async, &plugin_name)
                    .await
                {
                    Ok(msg) => {
                        let _ = notify_tx
                            .send(crate::plugin::manager::PluginRequest::Notify {
                                title: t("plugin_dev_toast_submitted_title"),
                                msg,
                                level: "info".to_string(),
                            })
                            .await;
                        developer_tool::progress_finish(
                            Some(tx_for_async),
                            Some(t("plugin_dev_fork_push_bg").to_string()),
                            None,
                        );
                    }
                    Err(e) => {
                        let _ = notify_tx
                            .send(crate::plugin::manager::PluginRequest::Notify {
                                title: t("plugin_dev_toast_submit_fail_title"),
                                msg: format!("{:?}", e),
                                level: "error".to_string(),
                            })
                            .await;
                        developer_tool::progress_finish(
                            Some(tx_for_async),
                            None,
                            Some(format!("{:?}", e)),
                        );
                    }
                }
            });
        });
        *installed = super::super::reload_installed_plugins(context, &None);
    }
}
