use super::reload_installed_plugins;
use super::toast::{Toast, spawn_with_toast};
use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::config::localization::t;
use crate::plugin::manager::toggles::{self, PluginToggle};
use crossterm::event::{KeyCode, KeyEvent};

/// The registry key and signature policy of the user's settings.
pub(super) fn registry_trust(context: &AppContext) -> crate::plugin::updater::RegistryTrust {
    crate::plugin::updater::RegistryTrust::from_settings(&context.config.settings)
}

/// Flips trust or enabled for the plugin under the cursor (saved and
/// applied by reloading the plugins) and refreshes the rows.
fn toggle(
    context: &mut AppContext,
    installed: &mut Vec<crate::plugin::installed::InstalledPlugin>,
    cursor_idx: usize,
    what: PluginToggle,
) {
    if let Some(name) = installed.get(cursor_idx).map(|p| p.name.clone()) {
        toggles::flip(&mut context.config, &name, what);
        *installed = reload_installed_plugins(context, &None);
    }
}

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
            toggle(context, installed, *cursor_idx, PluginToggle::Trust)
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            toggle(context, installed, *cursor_idx, PluginToggle::Enabled)
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            toggles::flip_secure_mode(&mut context.config);
            *installed = reload_installed_plugins(context, &None);
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
                let trust = registry_trust(context);
                spawn_with_toast(
                    async move { crate::plugin::updater::update(Some(&name), &trust).await },
                    ok,
                    err,
                );
            }
        }
        KeyCode::Char('U') => {
            let trust = registry_trust(context);
            spawn_with_toast(
                async move { crate::plugin::updater::update(None, &trust).await },
                Toast {
                    title: t("plugin_toast_update_all_title"),
                    msg: t("plugin_toast_update_all_ok"),
                },
                Toast {
                    title: t("plugin_toast_update_all_err_title"),
                    msg: t("plugin_toast_update_all_err"),
                },
            )
        }
        _ => {}
    }
}
