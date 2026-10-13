use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::app::state::ConfigDraft;
use crate::app::sys_helpers::{change_preset, change_theme};

pub fn apply_settings(state: &mut AppState, context: &mut AppContext, draft: ConfigDraft) {
    let ConfigDraft {
        settings,
        keymap_preset,
    } = draft;
    if settings.theme != context.config.settings.theme {
        change_theme(context, state, &settings.theme);
    }
    if keymap_preset != context.config.keybindings.preset {
        change_preset(context, &keymap_preset);
    }
    state.apply_settings(&settings);
    let lang_to_load = settings.language.clone();
    context.config.settings = settings;
    context.config.save_logging();
    crate::config::localization::load_language(&lang_to_load);
    state.refresh_both_panels(context.config.settings.show_hidden);
    let report = context.resolver.load_report();
    if !report.ok() {
        state
            .dialogs
            .replace(crate::app::state::PopupType::InfoPanel {
                lines: report.detail_lines(),
            });
    } else {
        state.dialogs.clear();
    }
}
