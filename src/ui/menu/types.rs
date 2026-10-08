use crate::config::localization::t;
use crate::keybindings::{Action, KeybindingResolver};

#[derive(Debug, Clone)]
pub struct MenuItemData {
    pub label: String,
    pub shortcut: String,
    pub active: bool,
    pub is_separator: bool,
    pub action: Option<Action>,
    pub submenu_idx: Option<usize>,
}

impl MenuItemData {
    pub fn new(label: String, shortcut: &str, active: bool) -> Self {
        Self {
            label,
            shortcut: shortcut.to_string(),
            active,
            is_separator: false,
            action: None,
            submenu_idx: None,
        }
    }
    pub fn with_action(mut self, action: Action) -> Self {
        self.action = Some(action);
        self
    }
    pub fn with_submenu(mut self, submenu_idx: usize) -> Self {
        self.submenu_idx = Some(submenu_idx);
        self
    }
    pub fn separator() -> Self {
        Self {
            is_separator: true,
            ..Self::new(String::new(), "", false)
        }
    }
}

/// A menu row that runs an action: label key, action, and the shortcut shown
/// when the keymap does not bind the action.
pub type ActionRow = (&'static str, Action, &'static str);

/// Builds a menu (Builder): rows take their shortcut from the live keymap.
pub struct MenuBuilder<'a> {
    resolver: &'a KeybindingResolver,
    items: Vec<MenuItemData>,
}

impl<'a> MenuBuilder<'a> {
    pub fn new(resolver: &'a KeybindingResolver) -> Self {
        Self {
            resolver,
            items: Vec::new(),
        }
    }

    /// The keymap's chord for `action`, or `fallback`.
    fn shortcut(&self, action: Action, fallback: &str) -> String {
        self.resolver
            .key_for_action(action)
            .unwrap_or(fallback)
            .to_string()
    }

    /// A row running `action`, marked when `active`.
    pub fn toggle(mut self, (label, action, fallback): ActionRow, active: bool) -> Self {
        let shortcut = self.shortcut(action, fallback);
        self.items
            .push(MenuItemData::new(t(label), &shortcut, active).with_action(action));
        self
    }

    pub fn action(self, row: ActionRow) -> Self {
        self.toggle(row, false)
    }

    /// Several action rows.
    pub fn actions(self, rows: &[ActionRow]) -> Self {
        rows.iter().fold(self, |menu, row| menu.action(*row))
    }

    /// Action rows only when `enabled`.
    pub fn actions_if(self, enabled: bool, rows: &[ActionRow]) -> Self {
        if enabled { self.actions(rows) } else { self }
    }

    /// A row with a fixed shortcut text and no action (or one set later).
    pub fn plain(mut self, label: &str, shortcut: &str, action: Option<Action>) -> Self {
        let mut item = MenuItemData::new(t(label), shortcut, false);
        item.action = action;
        self.items.push(item);
        self
    }

    /// `"Label >"` opening submenu `idx`.
    pub fn submenu(mut self, label: &str, idx: usize) -> Self {
        self.items
            .push(MenuItemData::new(format!("{} >", t(label)), "", false).with_submenu(idx));
        self
    }

    pub fn separator(mut self) -> Self {
        self.items.push(MenuItemData::separator());
        self
    }

    pub fn build(self) -> Vec<MenuItemData> {
        self.items
    }
}
