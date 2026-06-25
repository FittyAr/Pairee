use crate::keybindings::Action;

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
            label: String::new(),
            shortcut: String::new(),
            active: false,
            is_separator: true,
            action: None,
            submenu_idx: None,
        }
    }
}
