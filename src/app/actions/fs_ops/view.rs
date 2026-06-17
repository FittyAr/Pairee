use super::helper::command_exists;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType, Screen};
use crate::config::localization::t;
use crate::keybindings::Action;
use crate::terminal::TerminalBackend;

pub fn handle(
    state: &mut AppState,
    action: &Action,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active
        .entries
        .get(active.cursor_index)
        .filter(|e| !e.is_dir)
    {
        let path = entry.path.clone();
        let entry_name = entry.name.clone();
        state.push_file_view_history(path.clone());

        let rule = crate::config::associations::AssociationsConfig::load()
            .find_rule(&entry_name)
            .cloned();

        let mut ran_external = false;

        // Decide whether we want to use the external command.
        // If viewer_use_external is true:
        //   F3 (Action::View) uses external command.
        //   Alt+F3 (Action::ViewAlt) uses internal viewer.
        // If viewer_use_external is false (default):
        //   F3 (Action::View) uses internal viewer.
        //   Alt+F3 (Action::ViewAlt) uses external command.
        let use_external = match action {
            Action::View => context.config.settings.viewer_use_external,
            Action::ViewAlt => !context.config.settings.viewer_use_external,
            _ => false,
        };

        if use_external {
            if let Some(ref r) = rule {
                let cmd = r.resolve_view_cmd(&path);
                if command_exists(&cmd) {
                    ran_external = true;
                    if let Err(e) = crate::app::actions::exec::execute_external_command(
                        &path,
                        &cmd,
                        context,
                        terminal_backend,
                    ) {
                        state.active_popup = Some(PopupType::Error(format!(
                            "{} {}",
                            t("error_viewer_failed"),
                            e
                        )));
                    }
                }
            }
        }

        if !ran_external {
            let viewer = crate::ui::viewer::ViewerState::load(path);
            state.push_screen(Screen::Viewer(viewer));
        }
    }
    true
}
