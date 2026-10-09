//! File clipboard actions: yank / cut the targeted items, then paste them
//! (copy, move, overwrite or as links) into the active panel's folder.

use super::link;
use crate::app::context::AppContext;
use crate::app::state::file_clipboard::{ClipMode, FileClipboard};
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::fs::LinkKind;
use crate::fs::transfer::job::TransferOperation;
use crate::fs::transfer::{submit_simple, transfer_options_from_settings};
use crate::keybindings::Action;

pub fn handle(state: &mut AppState, action: &Action, context: &AppContext) -> bool {
    match action {
        Action::Yank => store(state, ClipMode::Copy),
        Action::Cut => store(state, ClipMode::Cut),
        Action::Paste => paste(state, context, false),
        Action::PasteOverwrite => paste(state, context, true),
        Action::PasteAsLink => paste_links(state, context),
        Action::ClearClipboard => state.file_clipboard = None,
        _ => return false,
    }
    true
}

fn store(state: &mut AppState, mode: ClipMode) {
    let panel = state.get_active_panel();
    let paths = panel.get_targeted_paths();
    if !paths.is_empty() {
        state.file_clipboard = Some(FileClipboard {
            paths,
            mode,
            source: panel.source.clone(),
        });
    }
}

/// Copies or moves the clipboard into the active folder. Copies pasted
/// next to their originals keep both (renamed); cut items pasted into their
/// own folder stay put.
fn paste(state: &mut AppState, context: &AppContext, overwrite: bool) {
    let Some(clip) = state.file_clipboard.clone() else {
        return;
    };
    let active = state.get_active_panel();
    let dest = active.current_path.clone();
    let in_place = clip
        .paths
        .iter()
        .all(|p| p.parent() == Some(dest.as_path()));
    if in_place && clip.mode == ClipMode::Cut {
        return;
    }
    let mut options = transfer_options_from_settings(&context.config.settings);
    if overwrite {
        options.conflict_resolution = "overwrite".into();
    } else if in_place {
        options.conflict_resolution = "rename".into();
    }
    let operation = match clip.mode {
        ClipMode::Copy => TransferOperation::Copy,
        ClipMode::Cut => TransferOperation::Move,
    };
    let dst_ssh = active.source.ssh().cloned();
    submit_simple(
        state,
        operation,
        clip.paths,
        dest,
        options,
        clip.source.ssh().cloned(),
        dst_ssh,
    );
    if clip.mode == ClipMode::Cut {
        state.file_clipboard = None;
    }
}

/// Symbolic links to the clipboard's items (local disk only).
fn paste_links(state: &mut AppState, context: &AppContext) {
    let Some(clip) = state.file_clipboard.clone() else {
        return;
    };
    if !clip.source.is_local() || !state.get_active_panel().source.is_local() {
        state
            .dialogs
            .replace(PopupType::Info(t("vfs_action_unsupported")));
        return;
    }
    let dest_dir = state.get_active_panel().current_path.clone();
    for src in &clip.paths {
        let Some(name) = src.file_name() else {
            continue;
        };
        if let Err(e) = link::make(state, src, &dest_dir.join(name), LinkKind::Symbolic) {
            state
                .dialogs
                .replace(PopupType::Error(format!("Link failed: {e}")));
            break;
        }
    }
    state.refresh_both_panels(context.config.settings.show_hidden);
}
