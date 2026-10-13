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
use std::time::{Duration, Instant};

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
    /// A panel whose view the drag scrolled (pointer on its first or last
    /// shown row), and the first entry it shows. Only the view moves: the
    /// panel's cursor stays, and the view follows it again after the drop.
    peek: Option<(ActivePanel, usize)>,
    /// When the view last scrolled, to scroll at a steady pace.
    scrolled_at: Option<Instant>,
}

/// Pause between two rows of automatic scrolling.
const SCROLL_EVERY: Duration = Duration::from_millis(100);

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

    /// First entry panel `side` shows while the drag scrolls it.
    pub fn peek_for(&self, side: ActivePanel) -> Option<usize> {
        self.peek
            .filter(|(s, _)| *s == side)
            .map(|(_, start)| start)
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
        peek: None,
        scrolled_at: None,
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
    autoscroll(state, Instant::now());
    true
}

/// Scrolls the view of the panel under the pointer by one row when the
/// pointer rests on its first or last shown row (called on every pointer
/// move and timer tick, at most once per [`SCROLL_EVERY`]). Returns whether
/// the view moved.
pub fn autoscroll(state: &mut AppState, now: Instant) -> bool {
    let Some(drag) = state.mouse.file_drag.as_ref() else {
        return false;
    };
    let Some((x, y)) = drag.at else {
        return false;
    };
    if drag
        .scrolled_at
        .is_some_and(|last| now.saturating_duration_since(last) < SCROLL_EVERY)
    {
        return false;
    }
    let Some(HitTarget::PanelRow { side, index }) = state.hits.at(x, y).map(|hit| hit.target)
    else {
        return false;
    };
    let shown = state.hits.collect(|target| match *target {
        HitTarget::PanelRow { side: s, index } if s == side => Some(index),
        _ => None,
    });
    let (Some(&first), Some(&last)) = (shown.iter().min(), shown.iter().max()) else {
        return false;
    };
    let len = state.panels.tabs(side).panel().entries.len();
    let start = if index == first && first > 0 {
        first - 1
    } else if index == last && last + 1 < len {
        first + 1
    } else {
        return false;
    };
    if let Some(drag) = state.mouse.file_drag.as_mut() {
        drag.peek = Some((side, start));
        drag.scrolled_at = Some(now);
    }
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
            if !state.cli.field.is_empty() && !state.cli.text().ends_with(' ') {
                state.cli.field.insert_char(' ');
            }
            state.cli.field.insert_str(&quoted.join(" "));
            state.cli.focused = true;
        }
        None => {}
    }
    true
}
