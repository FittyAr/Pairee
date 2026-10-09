use super::types::{MenuBuilder, MenuItemData};
use crate::keybindings::{Action, KeybindingResolver};

pub fn get_items(resolver: &KeybindingResolver) -> Vec<MenuItemData> {
    MenuBuilder::new(resolver)
        .actions(&[
            ("menu_help", Action::Help, "F1"),
            ("menu_about", Action::About, ""),
        ])
        .separator()
        .action(("menu_configuration", Action::SystemSettings, ""))
        .action(("menu_keyboard_shortcuts", Action::WhichKey, ""))
        .plain("menu_check_updates", "", Some(Action::CheckForUpdates))
        .separator()
        .action(("menu_save_setup", Action::SaveSetup, "Shf+F9"))
        .build()
}
