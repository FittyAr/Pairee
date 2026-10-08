//! Session lifecycle: what the panels show at startup and what is kept on
//! exit. "Save setup" (`config.toml`) only holds the defaults of new
//! panels; the last session (`session.toml`) restores tabs, folders and
//! layout on top of them, and command-line folders win over both.

pub mod capture;
pub mod cwd;
pub mod remote;
pub mod restore;
#[cfg(test)]
mod tests;

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::config::history::HistoryStore;
use crate::config::session::SessionFile;
use crate::config::settings::Settings;
use std::path::PathBuf;

/// Sets up the panels at startup: "Save setup" defaults, then the last
/// session (when `restore_session` is on), then the command-line folders.
/// Returns the notices to show once the interface is up.
pub fn start(state: &mut AppState, settings: &Settings, cli_paths: &[PathBuf]) -> Vec<String> {
    let session = settings.restore_session.then(SessionFile::load).flatten();
    start_with(state, settings, session.as_ref(), cli_paths)
}

/// [`start`] with an already loaded session.
pub fn start_with(
    state: &mut AppState,
    settings: &Settings,
    session: Option<&SessionFile>,
    cli_paths: &[PathBuf],
) -> Vec<String> {
    crate::app::sys_helpers::apply_setup(state, settings);
    let notices = session
        .map(|session| restore::restore(state, session, settings))
        .unwrap_or_default();
    restore::apply_start_paths(state, cli_paths);
    notices
}

/// Saves what outlives the run: "Save setup" when `auto_save_setup` is on,
/// the history lists the user keeps, and the session. Returns the folder a
/// shell wrapper should change to.
pub fn persist_on_exit(state: &AppState, context: &mut AppContext) -> Option<PathBuf> {
    let settings = &mut context.config.settings;
    if settings.auto_save_setup {
        crate::app::sys_helpers::capture_setup(state, settings);
        context.config.save_logging();
    }
    let settings = &context.config.settings;
    if let Err(e) = persisted_history(state.history.to_store(), settings).save() {
        log::warn!("Failed to save the history: {e}");
    }
    if settings.restore_session
        && let Err(e) = capture::capture(state, &settings.ssh_presets).save()
    {
        log::warn!("Failed to save the session: {e}");
    }
    cwd::exit_dir(state.get_active_panel())
}

/// Drops the history categories whose `save_*_history` setting is disabled.
pub fn persisted_history(store: HistoryStore, settings: &Settings) -> HistoryStore {
    store.retain_enabled(
        settings.save_commands_history,
        settings.save_folders_history,
        settings.save_view_and_edit_history,
    )
}
