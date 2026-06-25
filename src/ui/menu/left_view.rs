use super::types::MenuItemData;
use crate::app::state::{AppState, PanelViewMode};
use crate::config::localization::t;
use crate::keybindings::{Action, KeybindingResolver};

pub fn get_items(state: &AppState, resolver: &KeybindingResolver) -> Vec<MenuItemData> {
    let shortcut_for = |action: Action, fallback: &str| -> String {
        resolver
            .key_for_action(action)
            .map(|s| s.to_string())
            .unwrap_or_else(|| fallback.to_string())
    };

    vec![
        MenuItemData::new(
            t("menu_brief"),
            &shortcut_for(Action::PanelViewBrief, "Ctrl+1"),
            state.left_panel.view_mode == PanelViewMode::Brief,
        )
        .with_action(Action::PanelViewBrief),
        MenuItemData::new(
            t("menu_medium"),
            &shortcut_for(Action::PanelViewMedium, "Ctrl+2"),
            state.left_panel.view_mode == PanelViewMode::Medium,
        )
        .with_action(Action::PanelViewMedium),
        MenuItemData::new(
            t("menu_full"),
            &shortcut_for(Action::PanelViewFull, "Ctrl+3"),
            state.left_panel.view_mode == PanelViewMode::Full,
        )
        .with_action(Action::PanelViewFull),
        MenuItemData::new(
            t("menu_wide"),
            &shortcut_for(Action::PanelViewWide, "Ctrl+4"),
            state.left_panel.view_mode == PanelViewMode::Wide,
        )
        .with_action(Action::PanelViewWide),
        MenuItemData::new(
            t("menu_detailed"),
            &shortcut_for(Action::PanelViewDetailed, "Ctrl+5"),
            state.left_panel.view_mode == PanelViewMode::Detailed,
        )
        .with_action(Action::PanelViewDetailed),
        MenuItemData::new(
            t("menu_descriptions"),
            &shortcut_for(Action::PanelViewDescriptions, "Ctrl+6"),
            state.left_panel.view_mode == PanelViewMode::Descriptions,
        )
        .with_action(Action::PanelViewDescriptions),
        MenuItemData::new(
            t("menu_file_owners"),
            &shortcut_for(Action::PanelViewFileOwners, "Ctrl+7"),
            state.left_panel.view_mode == PanelViewMode::FileOwners,
        )
        .with_action(Action::PanelViewFileOwners),
        MenuItemData::new(
            t("menu_file_links"),
            &shortcut_for(Action::PanelViewFileLinks, "Ctrl+8"),
            state.left_panel.view_mode == PanelViewMode::FileLinks,
        )
        .with_action(Action::PanelViewFileLinks),
        MenuItemData::new(
            t("menu_alt_full"),
            &shortcut_for(Action::PanelViewAltFull, "Ctrl+9"),
            state.left_panel.view_mode == PanelViewMode::AltFull,
        )
        .with_action(Action::PanelViewAltFull),
    ]
}
