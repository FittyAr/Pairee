use super::reload_installed_plugins;
use super::toast::{Toast, spawn_with_toast};
use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::popup::{ConfirmUninstallPlugin, PluginMenuState};
use crate::app::state::{AppState, PopupType};
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

/// Uninstalls `name` (asked first by `ConfirmUninstallPlugin`) and
/// refreshes the Plugin Manager's Installed rows when it is on top.
pub fn uninstall(state: &mut AppState, context: &mut AppContext, name: &str) {
    match crate::plugin::updater::remove(name) {
        Ok(_) => {
            // `remove` cleared config.toml; drop the copy held in memory too
            // so a later save cannot bring the trust entry back.
            toggles::forget(&mut context.config, name);
            let name = name.to_string();
            tokio::spawn(async move {
                crate::plugin::registry::unregister_plugin(&name).await;
            });
        }
        Err(e) => {
            state.dialogs.push(PopupType::error(
                t("plugin_uninstall_failed")
                    .replacen("{}", name, 1)
                    .replacen("{}", &e.to_string(), 1),
            ));
            return;
        }
    }
    if let Some(menu) = state.dialogs.top_mut_as::<PluginMenuState>() {
        menu.installed = reload_installed_plugins(context, &None);
        menu.cursor_idx = menu.cursor_idx.min(menu.installed.len().saturating_sub(1));
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
                // A corrupt lockfile is left alone (`pairee plugin verify`
                // reports it).
                if let Ok(mut lock) = crate::plugin::updater::try_read_lockfile() {
                    if let Some(p) = lock.plugins.get_mut(name) {
                        p.pinned = !p.pinned;
                    }
                    let _ = crate::plugin::updater::write_lockfile(&lock);
                }

                *installed = reload_installed_plugins(context, &None);
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

/// The uninstall key (`d` / `D` / Del) asks [`ConfirmUninstallPlugin`] for
/// the plugin under the cursor. Returns whether `key` was that key.
pub fn ask_uninstall(state: &mut AppState, key: KeyEvent) -> bool {
    if !matches!(key.code, KeyCode::Char('d' | 'D') | KeyCode::Delete) {
        return false;
    }
    let name = state
        .dialogs
        .top_as::<PluginMenuState>()
        .and_then(|menu| menu.installed.get(menu.cursor_idx))
        .map(|p| p.name.clone());
    if let Some(name) = name {
        state.dialogs.push(ConfirmUninstallPlugin { name });
    }
    true
}
