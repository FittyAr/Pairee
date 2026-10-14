//! Contents of dialogs read in the background, so a slow disk, a network
//! share or a huge archive never freezes the UI: the folder tree (F10 /
//! Tree view) and the archive commands' listing and test. The dialog opens
//! at once (the tree shows "Reading…", the archive a progress message) and
//! is filled in when the job ends; a dialog closed meanwhile drops it.

use crate::app::jobs::JobSlot;
use crate::app::state::popup::{InfoMessage, TreeView};
use crate::app::state::types::{TreeNode, TreeViewCaller};
use crate::app::state::{AppState, PopupType, Screen};
use crate::config::localization::t;
use std::path::PathBuf;

/// What a background read produced.
#[derive(Debug)]
enum Loaded {
    Tree {
        root: PathBuf,
        nodes: Vec<TreeNode>,
    },
    Archive {
        path: PathBuf,
        what: ArchiveRead,
        listing: Result<Vec<String>, String>,
    },
}

/// Why an archive is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveRead {
    /// Show its contents in the viewer.
    List,
    /// Only check it reads (integrity test).
    Test,
}

/// Depth of the folder tree below its root.
const TREE_DEPTH: usize = 3;

/// The one background read of dialog contents (opening another supersedes
/// it: these dialogs are modal).
#[derive(Debug, Default)]
pub struct DialogLoads {
    job: JobSlot<Loaded>,
}

impl DialogLoads {
    pub fn is_running(&self) -> bool {
        self.job.is_running()
    }
}

/// Opens the tree view of `root` for `caller` (pushed over the dialog that
/// asked, or as the only dialog) and reads the tree in the background.
pub fn open_tree(state: &mut AppState, root: PathBuf, caller: TreeViewCaller, push: bool) {
    let tree = TreeView {
        root: root.clone(),
        nodes: Vec::new(),
        loading: true,
        cursor_idx: 0,
        caller,
    };
    if push {
        state.dialogs.push(tree);
    } else {
        state.dialogs.replace(tree);
    }
    state.dialog_loads.job.start(move |_| {
        let nodes = crate::app::sys_helpers::build_tree_nodes(&root, 0, TREE_DEPTH);
        Loaded::Tree { root, nodes }
    });
}

/// Reads the archive at `path` in the background, showing a message until
/// it is done.
pub fn read_archive(state: &mut AppState, path: PathBuf, what: ArchiveRead) {
    state.dialogs.replace(PopupType::info(reading_message()));
    state.dialog_loads.job.start(move |_| {
        let listing = crate::fs::archive::list_archive_files(&path).map_err(|e| e.to_string());
        Loaded::Archive {
            path,
            what,
            listing,
        }
    });
}

fn reading_message() -> String {
    t("archive_reading")
}

/// Puts a finished read in its dialog. Returns whether one arrived.
pub fn poll(state: &mut AppState) -> bool {
    let loaded = match state.dialog_loads.job.poll() {
        Some(loaded) => loaded,
        None if state.dialog_loads.job.take_panicked() => {
            failed(state);
            return true;
        }
        None => return false,
    };
    match loaded {
        Loaded::Tree { root, nodes } => {
            if let Some(tree) = loading_tree(state, &root) {
                tree.nodes = nodes;
                tree.loading = false;
            }
        }
        Loaded::Archive {
            path,
            what,
            listing,
        } => {
            if !archive_message_on_top(state) {
                return true;
            }
            state.dialogs.clear();
            match (what, listing) {
                (ArchiveRead::List, Ok(list)) => {
                    let viewer = crate::ui::viewer::ViewerState::from_text(path, list);
                    state.push_screen(Screen::Viewer(viewer));
                }
                (ArchiveRead::Test, Ok(_)) => {
                    state.dialogs.replace(PopupType::info(t("archive_test_ok")))
                }
                (ArchiveRead::List, Err(e)) => state.dialogs.replace(PopupType::error(
                    t("archive_list_failed").replacen("{}", &e, 1),
                )),
                (ArchiveRead::Test, Err(e)) => state.dialogs.replace(PopupType::error(
                    t("archive_test_failed").replacen("{}", &e, 1),
                )),
            }
        }
    }
    true
}

/// A job that panicked: the tree shows what it has, the archive message
/// becomes an error.
fn failed(state: &mut AppState) {
    for dialog in state.dialogs.iter_mut() {
        if let PopupType::TreeView(tree) = dialog {
            tree.loading = false;
        }
    }
    if archive_message_on_top(state) {
        state
            .dialogs
            .replace(PopupType::error(t("background_job_failed")));
    }
}

/// The open tree view of `root` still waiting for its nodes.
fn loading_tree<'a>(state: &'a mut AppState, root: &PathBuf) -> Option<&'a mut TreeView> {
    state.dialogs.iter_mut().find_map(|dialog| match dialog {
        PopupType::TreeView(tree) if tree.loading && tree.root == *root => Some(tree),
        _ => None,
    })
}

/// Whether the "reading the archive" message is still shown (not closed
/// by the user).
fn archive_message_on_top(state: &AppState) -> bool {
    state
        .dialogs
        .top_as::<InfoMessage>()
        .is_some_and(|InfoMessage(text)| *text == reading_message())
}
