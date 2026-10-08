use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::fs::FileEntry;
use crate::fs::vfs::PanelSource;

/// Enters the highlighted folder or archive, or opens the highlighted file
/// (internal viewer, or the associated / system program).
pub fn handle_enter_key(state: &mut AppState, context: &crate::app::context::AppContext) {
    enter(state, context, false);
}

/// Ctrl+PgDn: enters the highlighted folder or archive; files are left alone.
pub fn handle_open_archive_key(state: &mut AppState, context: &crate::app::context::AppContext) {
    enter(state, context, true);
}

/// Opens the highlighted folder or archive in a new tab next to the
/// current one; files are left alone.
pub fn handle_open_in_new_tab_key(state: &mut AppState, context: &crate::app::context::AppContext) {
    let Some(entry) = cursor_entry(state).filter(is_enterable) else {
        return;
    };
    let show_hidden = context.config.settings.show_hidden;
    // An archive that cannot be browsed here only shows why, in place.
    if entry.is_dir || browse_refusal(&state.get_active_panel().source).is_none() {
        state.duplicate_active_tab(show_hidden);
    }
    enter(state, context, true);
    state.refresh_active_panel(show_hidden);
}

fn cursor_entry(state: &AppState) -> Option<FileEntry> {
    let active = state.get_active_panel();
    active.entries.get(active.cursor_index).cloned()
}

/// Folders and browsable archives open in the panel.
fn is_enterable(entry: &FileEntry) -> bool {
    entry.is_dir || crate::fs::archive::is_browsable(&entry.path)
}

fn enter(state: &mut AppState, context: &crate::app::context::AppContext, folders_only: bool) {
    let Some(entry) = cursor_entry(state) else {
        return;
    };
    let active = state.get_active_panel();
    if !is_enterable(&entry) {
        if !folders_only {
            open_file(state, context, &entry);
        }
    } else if entry.is_dir {
        change_dir(state, entry.path);
    } else {
        match browse_refusal(&active.source) {
            None => change_dir(state, entry.path),
            Some(key) => state.dialogs.replace(PopupType::Info(t(key))),
        }
    }
}

/// Why an archive on this source cannot be opened as a folder.
fn browse_refusal(source: &PanelSource) -> Option<&'static str> {
    match source {
        PanelSource::Local => None,
        PanelSource::Remote(_) => Some("archive_remote_unsupported"),
        PanelSource::Archive(_) => Some("archive_nested_unsupported"),
    }
}

/// Points the active panel at `dir` (the caller rereads it).
fn change_dir(state: &mut AppState, dir: std::path::PathBuf) {
    state.push_folders_history(dir.clone());
    let active_mut = state.get_active_panel_mut();
    // Going up via "..": land on the folder (or archive) we just left.
    active_mut.pending_focus = child_name_of(&dir, &active_mut.current_path);
    active_mut.current_path = dir;
    active_mut.cursor_index = 0;
    active_mut.clear_selection();
}

fn open_file(state: &mut AppState, context: &crate::app::context::AppContext, entry: &FileEntry) {
    let settings = &context.config.settings;
    // Programs can only open real local files.
    let local = state.get_active_panel().source.capabilities().local_tools;
    if !settings.enter_use_external || !local {
        state.push_file_view_history(entry.path.clone());
        state.open_viewer(entry.path.clone(), settings, false);
        return;
    }
    let rule = crate::config::associations::AssociationsConfig::load()
        .find_rule(&entry.name)
        .cloned();
    if let Some(r) = rule {
        // Association: parse command into (program, args) and exec directly
        // without a shell. The file path is passed as a single argv entry,
        // so a malicious filename cannot inject shell commands.
        let (program, args) = r.resolve_open_cmd(&entry.path);
        if !program.is_empty() {
            if settings.automatic_update_env_variables {
                crate::app::sys_helpers::refresh_env_vars();
            }
            let _ = std::process::Command::new(&program).args(&args).spawn();
        }
    } else if !cfg!(target_os = "windows") || settings.use_windows_registered_types {
        // No matching association: hand the path to the OS-registered
        // handler (ShellExecuteW / xdg-open / open) without a shell, so the
        // name is never parsed.
        if settings.automatic_update_env_variables {
            crate::app::sys_helpers::refresh_env_vars();
        }
        crate::shell::open_with_system_handler(&entry.path);
    }
}

/// Name of the direct child of `parent` that `child` is (e.g. the folder we
/// leave when ascending), or `None` when `child` is not directly below it.
fn child_name_of(parent: &std::path::Path, child: &std::path::Path) -> Option<String> {
    (child.parent() == Some(parent))
        .then(|| child.file_name())
        .flatten()
        .map(|n| n.to_string_lossy().into_owned())
}

/// Ascends to parent folder directory.
pub fn handle_backspace_key(state: &mut AppState, show_hidden: bool) {
    let parent_path = state
        .get_active_panel()
        .current_path
        .parent()
        .map(|p| p.to_path_buf());
    if let Some(parent) = parent_path {
        change_dir(state, parent);
        state.refresh_active_panel(show_hidden);
    }
}
