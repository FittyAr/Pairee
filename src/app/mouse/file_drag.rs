//! Dragging files out of a panel and dropping them on the other panel, on
//! a folder, on a tab or on the command line. A drop copies (Shift: moves)
//! through the same dialog as F5 / F6, so filters, conflicts, the queue and
//! SFTP all apply.

use crate::app::context::AppContext;
use crate::app::state::popup::TransferPromptOp;
use crate::app::state::{ActivePanel, AppState};
use crate::ui::hit::HitTarget;
use crossterm::event::KeyModifiers;
use std::path::{Path, PathBuf};

/// A press on a panel row that may turn into a drag.
#[derive(Debug, Clone)]
pub struct FileDrag {
    side: ActivePanel,
    index: usize,
    from: (u16, u16),
    /// Where the pointer is, once it moved off the pressed cell.
    at: Option<(u16, u16)>,
    modifiers: KeyModifiers,
    paths: Vec<PathBuf>,
}

/// Where a drop puts the files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drop {
    /// Into `dir`, shown by panel `side`.
    Folder { side: ActivePanel, dir: PathBuf },
    /// Onto the command line, as quoted paths.
    CommandLine,
}

impl FileDrag {
    /// What is being dragged, for the pointer label.
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// Pointer cell, when the drag is going on.
    pub fn pointer(&self) -> Option<(u16, u16)> {
        self.at
    }

    /// The operation a drop would run now.
    pub fn op(&self) -> TransferPromptOp {
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            TransferPromptOp::Move
        } else {
            TransferPromptOp::Copy
        }
    }
}

/// Remembers a plain left press on row `index` of panel `side`.
pub fn press(state: &mut AppState, side: ActivePanel, index: usize, at: (u16, u16)) {
    state.mouse.file_drag = Some(FileDrag {
        side,
        index,
        from: at,
        at: None,
        modifiers: KeyModifiers::NONE,
        paths: Vec::new(),
    });
}

/// Follows the pointer. Returns whether a drag is going on (so the screen
/// shows where the files would go).
pub fn follow(state: &mut AppState, at: (u16, u16), modifiers: KeyModifiers) -> bool {
    let Some(mut drag) = state.mouse.file_drag.take() else {
        return false;
    };
    if drag.at.is_none() {
        if at == drag.from {
            state.mouse.file_drag = Some(drag);
            return false;
        }
        drag.paths = payload(state, drag.side, drag.index);
        if drag.paths.is_empty() {
            return false;
        }
    }
    drag.at = Some(at);
    drag.modifiers = modifiers;
    state.mouse.file_drag = Some(drag);
    true
}

/// The tagged entries when the dragged row is tagged, else that row; never
/// `..`.
fn payload(state: &AppState, side: ActivePanel, index: usize) -> Vec<PathBuf> {
    let panel = state.panels.tabs(side).panel();
    let Some(entry) = panel.entries.get(index) else {
        return Vec::new();
    };
    if entry.name == ".." {
        return Vec::new();
    }
    if panel.selected_paths.contains(&entry.path) {
        panel
            .entries
            .iter()
            .filter(|e| panel.selected_paths.contains(&e.path))
            .map(|e| e.path.clone())
            .collect()
    } else {
        vec![entry.path.clone()]
    }
}

/// Where dropping on `target` would put the files, if anywhere.
pub fn drop_target(state: &AppState, drag: &FileDrag, target: Option<&HitTarget>) -> Option<Drop> {
    let folder = |side: ActivePanel, dir: PathBuf| Some(Drop::Folder { side, dir });
    let target = match target? {
        HitTarget::PanelRow { side, index } => {
            let panel = state.panels.tabs(*side).panel();
            match panel.entries.get(*index) {
                Some(entry) if entry.name == ".." => {
                    folder(*side, panel.current_path.parent()?.to_path_buf())
                }
                Some(entry) if entry.is_dir && !drag.paths.contains(&entry.path) => {
                    folder(*side, entry.path.clone())
                }
                _ if *side != drag.side => folder(*side, panel.current_path.clone()),
                _ => None,
            }
        }
        HitTarget::PanelBody(side)
        | HitTarget::PanelTitle(side)
        | HitTarget::PanelHeader { side, .. }
            if *side != drag.side =>
        {
            folder(*side, state.panels.tabs(*side).panel().current_path.clone())
        }
        HitTarget::Tab(id) => [ActivePanel::Left, ActivePanel::Right]
            .into_iter()
            .find_map(|side| {
                let tab = state.panels.tabs(side).find(*id)?;
                folder(side, tab.panel.current_path.clone())
            }),
        HitTarget::Action(crate::keybindings::Action::FocusCli) => Some(Drop::CommandLine),
        _ => None,
    }?;
    // Nothing to do where the files already are, or into themselves.
    match &target {
        Drop::Folder { dir, .. } if is_noop(&drag.paths, dir) => None,
        _ => Some(target),
    }
}

fn is_noop(paths: &[PathBuf], dir: &Path) -> bool {
    paths
        .iter()
        .any(|path| path.parent() == Some(dir) || dir.starts_with(path))
}

/// Ends the drag on `target`. Returns whether a drag was going on.
pub fn release(
    state: &mut AppState,
    context: &AppContext,
    target: Option<&HitTarget>,
    modifiers: KeyModifiers,
) -> bool {
    let Some(mut drag) = state.mouse.file_drag.take() else {
        return false;
    };
    if drag.at.is_none() {
        return false;
    }
    drag.modifiers = modifiers;
    match drop_target(state, &drag, target) {
        Some(Drop::Folder { side, dir }) => {
            let endpoints = (
                state.panels.tabs(drag.side).panel().source.ssh().cloned(),
                state.panels.tabs(side).panel().source.ssh().cloned(),
            );
            crate::app::actions::fs_ops::transfer::start(
                state,
                context,
                (drag.op(), drag.paths, dir),
                endpoints,
            );
        }
        Some(Drop::CommandLine) => {
            let quoted: Vec<String> = drag
                .paths
                .iter()
                .map(|path| {
                    let text = path.to_string_lossy();
                    shlex::try_quote(&text).map_or_else(|_| text.to_string(), |q| q.into_owned())
                })
                .collect();
            if !state.cli_input.is_empty() && !state.cli_input.ends_with(' ') {
                state.cli_input.push(' ');
            }
            state.cli_input.push_str(&quoted.join(" "));
            state.cli_focused = true;
        }
        None => {}
    }
    true
}
