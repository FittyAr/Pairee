use crate::app::context::AppContext;
use crate::app::state::AppState;

/// Changes the current configuration theme.
pub fn change_theme(context: &mut AppContext, state: &mut AppState, theme_name: &str) {
    context.config.settings.theme = theme_name.to_string();
    let theme = if theme_name == "classic_blue" {
        crate::config::theme::Theme::classic_blue()
    } else {
        crate::config::theme::Theme::default()
    };
    context.config.theme = theme;
    context.config.save_logging();
    state.refresh_both_panels(context.config.settings.show_hidden);
}

/// Changes the current keybinding preset.
pub fn change_preset(context: &mut AppContext, preset_name: &str) {
    context.config.keybindings.preset = preset_name.to_string();
    context.config.settings.keybinding_preset = preset_name.to_string();
    context.resolver = crate::keybindings::KeybindingResolver::new(&context.config);
    context.config.save_logging();
}

/// Copies the session's panel layout (visibility plus the active panel's view
/// mode, sort order and long-names flag) into the settings before "Save setup"
/// (explicit or `auto_save_setup` on exit) writes them to disk.
pub fn capture_setup(state: &AppState, settings: &mut crate::config::settings::Settings) {
    settings.left_panel_visible = state.panels.left_visible;
    settings.right_panel_visible = state.panels.right_visible;
    let active = state.get_active_panel();
    settings.panel_view_mode = active.view_mode;
    settings.sort_field = active.sort_field;
    settings.sort_reverse = active.sort_reverse;
    settings.show_long_names = active.show_long_names;
}

/// Applies the saved panel layout to both panels at startup.
pub fn apply_setup(state: &mut AppState, settings: &crate::config::settings::Settings) {
    state.panels.left_visible = settings.left_panel_visible;
    state.panels.right_visible = settings.right_panel_visible;
    for (_, tab) in state.panels.all_tabs_mut() {
        let panel = &mut tab.panel;
        panel.view_mode = settings.panel_view_mode;
        panel.sort_field = settings.sort_field;
        panel.sort_reverse = settings.sort_reverse;
        panel.show_long_names = settings.show_long_names;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_visibility_round_trips_through_settings() {
        let mut state = AppState::new(".".into(), ".".into());
        let mut settings = crate::config::settings::Settings::default();
        state.panels.right_visible = false;
        state.get_active_panel_mut().sort_reverse = true;
        capture_setup(&state, &mut settings);
        assert!(settings.left_panel_visible);
        assert!(!settings.right_panel_visible);
        assert!(settings.sort_reverse);

        let mut fresh = AppState::new(".".into(), ".".into());
        apply_setup(&mut fresh, &settings);
        assert!(!fresh.panels.right_visible);
        assert!(
            fresh
                .panels
                .side(crate::app::state::ActivePanel::Left)
                .sort_reverse
                && fresh
                    .panels
                    .side(crate::app::state::ActivePanel::Right)
                    .sort_reverse
        );
    }
}
