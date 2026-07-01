use super::reload_installed_plugins;
use crate::app::context::AppContext;
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_installed(
    key: KeyEvent,
    context: &mut AppContext,
    cursor_idx: &mut usize,
    installed: &mut Vec<(String, String, bool, bool, Option<String>)>,
) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            if !installed.is_empty() {
                if *cursor_idx == 0 {
                    *cursor_idx = installed.len() - 1;
                } else {
                    *cursor_idx -= 1;
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            if !installed.is_empty() {
                if *cursor_idx + 1 >= installed.len() {
                    *cursor_idx = 0;
                } else {
                    *cursor_idx += 1;
                }
            }
        }
        KeyCode::Char('t') | KeyCode::Char('T') => {
            if let Some((name, _, _, _, _)) = installed.get(*cursor_idx) {
                if let Ok(mut config) = crate::config::AppConfig::load_or_create() {
                    let plugin_conf =
                        config
                            .settings
                            .plugins
                            .entry(name.clone())
                            .or_insert_with(|| crate::config::settings::PluginConfig {
                                name: name.clone(),
                                trusted: false,
                            });
                    plugin_conf.trusted = !plugin_conf.trusted;
                    let _ = config.save();
                }

                if let Ok(c) = crate::config::AppConfig::load_or_create() {
                    context.config = c;
                }

                *installed = reload_installed_plugins(context, &None);
            }
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            if let Some((name, _, _, _, _)) = installed.get(*cursor_idx) {
                let mut lock = crate::plugin::updater::read_lockfile();
                if let Some(p) = lock.plugins.get_mut(name) {
                    p.pinned = !p.pinned;
                }
                let _ = crate::plugin::updater::write_lockfile(&lock);

                *installed = reload_installed_plugins(context, &None);
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
            if let Some((name, _, _, _, _)) = installed.get(*cursor_idx) {
                let _ = crate::plugin::updater::remove(name);
                *installed = reload_installed_plugins(context, &None);
                *cursor_idx = (*cursor_idx).min(installed.len().saturating_sub(1));
            }
        }
        KeyCode::Char('u') => {
            if let Some((name, _, _, _, _)) = installed.get(*cursor_idx) {
                let name_clone = name.clone();
                let tx = crate::plugin::PluginManager::get_sender();
                tokio::spawn(async move {
                    match crate::plugin::updater::install(&name_clone, None).await {
                        Ok(_) => {
                            let _ = tx
                                .send(crate::plugin::manager::PluginRequest::Notify {
                                    title: t("plugin_toast_update_title"),
                                    msg: t("plugin_toast_update_ok").replace("{}", &name_clone),
                                    level: "info".to_string(),
                                })
                                .await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(crate::plugin::manager::PluginRequest::Notify {
                                    title: t("plugin_toast_update_err_title"),
                                    msg: t("plugin_toast_update_err")
                                        .replace("{}", &name_clone)
                                        .replace("{:?}", &format!("{:?}", e)),
                                    level: "error".to_string(),
                                })
                                .await;
                        }
                    }
                });
            }
        }
        KeyCode::Char('U') => {
            let tx = crate::plugin::PluginManager::get_sender();
            tokio::spawn(async move {
                match crate::plugin::updater::update(None).await {
                    Ok(_) => {
                        let _ = tx
                            .send(crate::plugin::manager::PluginRequest::Notify {
                                title: t("plugin_toast_update_all_title"),
                                msg: t("plugin_toast_update_all_ok"),
                                level: "info".to_string(),
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(crate::plugin::manager::PluginRequest::Notify {
                                title: t("plugin_toast_update_all_err_title"),
                                msg: t("plugin_toast_update_all_err")
                                    .replace("{:?}", &format!("{:?}", e)),
                                level: "error".to_string(),
                            })
                            .await;
                    }
                }
            });
        }
        _ => {}
    }
}
