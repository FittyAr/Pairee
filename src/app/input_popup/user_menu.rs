use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Clone)]
pub struct UserMenuItem {
    pub key: String,
    pub label: String,
    pub command: Option<String>,
    pub action: Option<Action>,
}

/// The menu shown while `usermenu.toml` defines no commands: key, label, action.
const DEFAULT_ITEMS: [(&str, &str, Action); 11] = [
    ("1", "user_cmd_refresh", Action::Refresh),
    ("2", "user_cmd_toggle_hidden", Action::ToggleHidden),
    ("3", "user_cmd_swap", Action::SwapPanels),
    ("4", "user_cmd_task_list", Action::TaskList),
    ("5", "user_cmd_git", Action::OpenGitPanel),
    ("6", "user_cmd_mkdir", Action::MkDir),
    ("T", "user_cmd_new_tab", Action::NewTab),
    ("O", "user_cmd_open_in_new_tab", Action::OpenInNewTab),
    ("W", "user_cmd_close_tab", Action::CloseTab),
    ("F", "user_cmd_quick_filter", Action::QuickFilter),
    ("H", "user_cmd_help", Action::Help),
];

pub fn get_user_menu_items() -> Vec<UserMenuItem> {
    let custom_cmds = crate::app::sys_helpers::load_user_menu_commands();
    let mut items = Vec::new();
    if !custom_cmds.is_empty() {
        for (k, v) in custom_cmds {
            items.push(UserMenuItem {
                key: k.clone(),
                label: v.clone(),
                command: Some(v),
                action: None,
            });
        }
    } else {
        items.extend(
            DEFAULT_ITEMS
                .iter()
                .map(|&(key, label, action)| UserMenuItem {
                    key: key.to_string(),
                    label: crate::config::localization::t(label),
                    command: None,
                    action: Some(action),
                }),
        );
    }
    // Always append Edit option at the end
    items.push(UserMenuItem {
        key: "E".to_string(),
        label: crate::config::localization::t("menu_edit_user_menu"),
        command: None,
        action: Some(Action::EditUserMenu),
    });
    items
}

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::UserMenu { cursor_idx }) = state.dialogs.top_mut() else {
        return Err(());
    };
    let items = get_user_menu_items();
    let chosen = match list_key(ListKeys::ARROWS_VIM, key.code, cursor_idx, items.len()) {
        ListKey::Moved => return Ok(None),
        ListKey::Close => None,
        ListKey::Activate(idx) => items.get(idx),
        // Any other character runs the item with that shortcut key.
        ListKey::Other => match key.code {
            KeyCode::Char(c) => {
                let shortcut = c.to_string().to_uppercase();
                match items.iter().find(|it| it.key.to_uppercase() == shortcut) {
                    Some(item) => Some(item),
                    None => return Ok(None),
                }
            }
            _ => return Err(()),
        },
    };
    state.dialogs.clear();
    match chosen {
        Some(item) => execute_item(state, context, item),
        None => Ok(None),
    }
}

fn execute_item(
    state: &mut AppState,
    context: &mut AppContext,
    item: &UserMenuItem,
) -> Result<Option<Action>, ()> {
    if let Some(act) = item.action {
        // Apply immediate settings checks for defaults
        if act == Action::Refresh {
            state.refresh_both_panels(context.config.settings.show_hidden);
            return Ok(None);
        } else if act == Action::ToggleHidden {
            context.config.settings.show_hidden = !context.config.settings.show_hidden;
            context.config.save_logging();
            state.refresh_both_panels(context.config.settings.show_hidden);
            return Ok(None);
        } else if act == Action::SwapPanels {
            state.swap_panels();
            return Ok(None);
        }
        return Ok(Some(act));
    }

    if let Some(cmd_template) = &item.command {
        let active_panel = state.get_active_panel();
        let highlighted = active_panel.entries.get(active_panel.cursor_index);
        // Substitute `{f}` (file name) and `{p}` (full path) with
        // shell-quoted values so a filename containing shell
        // metacharacters (`;`, `|`, `$`, `` ` ``, etc.) cannot break out
        // of the surrounding command and execute arbitrary code.
        // `execute_shell_command` later routes the resulting string
        // through the platform shell, so unquoted substitutions would be
        // a command-injection vector.
        let final_cmd = if let Some(e) = highlighted {
            // Single pass: substituted values are never rescanned, so a
            // name like `;id;{p}` cannot inject a second placeholder. On
            // Windows `%`, `!`, `^`, `&`... in names are caret-escaped.
            crate::shell::expand_placeholders(
                cmd_template,
                &e.name,
                &e.path.to_string_lossy(),
                crate::shell::quote_native,
            )
        } else {
            cmd_template.clone()
        };
        state.pending_custom_command = Some(final_cmd);
        return Ok(None);
    }

    Ok(None)
}
