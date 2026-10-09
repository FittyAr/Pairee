use super::reload_installed_plugins;
use super::toast::{Toast, spawn_with_toast};
use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_installed(
    key: KeyEvent,
    context: &mut AppContext,
    cursor_idx: &mut usize,
    installed: &mut Vec<crate::plugin::installed::InstalledPlugin>,
) {
    if list_key(ListKeys::ARROWS_VIM, key.code, cursor_idx, installed.len()) == ListKey::Moved {
        return;
    }
    match key.code {
        KeyCode::Char('t') | KeyCode::Char('T') => {
            if let Some(name) = installed.get(*cursor_idx).map(|p| &p.name) {
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
            if let Some(name) = installed.get(*cursor_idx).map(|p| &p.name) {
                let mut lock = crate::plugin::updater::read_lockfile();
                if let Some(p) = lock.plugins.get_mut(name) {
                    p.pinned = !p.pinned;
                }
                let _ = crate::plugin::updater::write_lockfile(&lock);

                *installed = reload_installed_plugins(context, &None);
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
            if let Some(name) = installed.get(*cursor_idx).map(|p| &p.name) {
                if crate::plugin::updater::remove(name).is_ok() {
                    let name = name.clone();
                    tokio::spawn(async move {
                        crate::plugin::registry::unregister_plugin(&name).await;
                    });
                }
                *installed = reload_installed_plugins(context, &None);
                *cursor_idx = (*cursor_idx).min(installed.len().saturating_sub(1));
            }
        }
        KeyCode::Char('u') => {
            if let Some(name) = installed.get(*cursor_idx).map(|p| p.name.clone()) {
                let ok = Toast {
                    title: t("plugin_toast_update_title"),
                    msg: t("plugin_toast_update_ok").replace("{}", &name),
                };
                let err = Toast {
                    title: t("plugin_toast_update_err_title"),
                    msg: t("plugin_toast_update_err").replace("{}", &name),
                };
                spawn_with_toast(
                    async move { crate::plugin::updater::install(&name, None).await },
                    ok,
                    err,
                );
            }
        }
        KeyCode::Char('U') => spawn_with_toast(
            crate::plugin::updater::update(None),
            Toast {
                title: t("plugin_toast_update_all_title"),
                msg: t("plugin_toast_update_all_ok"),
            },
            Toast {
                title: t("plugin_toast_update_all_err_title"),
                msg: t("plugin_toast_update_all_err"),
            },
        ),
        _ => {}
    }
}
