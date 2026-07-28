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
    let _ = context.config.save();
    state.refresh_both_panels(context.config.settings.show_hidden);
}

/// Changes the current keybinding preset.
pub fn change_preset(context: &mut AppContext, preset_name: &str) {
    context.config.keybindings.preset = preset_name.to_string();
    context.config.settings.keybinding_preset = preset_name.to_string();
    // `Arc::new` a fresh resolver: the preset change rebuilds
    // the static layers from scratch, and any plugin / Lua
    // bindings registered before the change are gone (the
    // plugin overlays in the old `Arc` are dropped here, so
    // the plugins must re-register). In practice plugins are
    // loaded once at startup so this is invisible to them;
    // for the user the practical effect is "preset change
    // resets the resolver", which matches the previous
    // behaviour.
    context.resolver =
        std::sync::Arc::new(crate::keybindings::KeybindingResolver::new(&context.config));
    let _ = context.config.save();
}
