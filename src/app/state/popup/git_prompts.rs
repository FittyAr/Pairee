//! Git dialogs opened over the Git panel (pushed on the dialog stack, so
//! closing one shows the panel again): commit, checkout, diff, name prompt,
//! confirmations, remotes, plus the clone form and the progress popup.

use crate::app::form::{FieldPair, FormLayout};
use crate::app::text_input::TextField;
use std::path::PathBuf;

/// Commit message (Ctrl+A switches to amending the last commit).
#[derive(Debug, Clone)]
pub struct GitCommitPromptState {
    pub input: TextField,
    pub repo_path: PathBuf,
    pub is_amend: bool,
}

/// Check out a branch or commit?
#[derive(Debug, Clone)]
pub struct GitConfirmCheckoutState {
    pub target: String,
    pub is_branch: bool,
    pub repo_path: PathBuf,
}

/// A diff (of a file or a commit), scrolled.
#[derive(Debug, Clone)]
pub struct GitDiffViewState {
    pub repo_path: PathBuf,
    pub file_path: Option<String>,
    pub commit_hash: Option<String>,
    pub diff_content: String,
    pub scroll_y: usize,
}

/// What a Git name prompt does with the typed name (Strategy for the shared
/// name prompt: branch create / rename, stash save, tag create).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitNameAction {
    CreateBranch { start_point: String },
    RenameBranch { old_name: String },
    SaveStash { include_untracked: bool },
    CreateTag { target: String },
}

/// One-line Git prompt (name, OK, Cancel) opened over the Git panel.
#[derive(Debug, Clone)]
pub struct GitNamePromptState {
    pub action: GitNameAction,
    pub input: TextField,
    /// Focused row: 0 = name, 1 = OK, 2 = Cancel.
    pub cursor_idx: usize,
    pub repo_path: PathBuf,
}

impl GitNamePromptState {
    /// Name field, OK, Cancel.
    pub const FORM: FormLayout = FormLayout::new(3, 1);
    pub const BUTTON_CANCEL: usize = 2;

    /// A prompt for `action`; renaming starts from the current name.
    pub fn new(action: GitNameAction, repo_path: PathBuf) -> Self {
        let input = match &action {
            GitNameAction::RenameBranch { old_name } => TextField::new(old_name.as_str()),
            _ => TextField::default(),
        };
        Self {
            action,
            input,
            cursor_idx: 0,
            repo_path,
        }
    }
}

/// Yes/No before a Git action (delete, merge, reset...).
#[derive(Debug, Clone)]
pub struct GitConfirmActionState {
    pub message: String,
    pub repo_path: PathBuf,
    pub action: crate::app::state::types::GitConfirmedAction,
}

/// The remotes of the repository: add (`a`) and delete.
#[derive(Debug, Clone)]
pub struct GitRemoteManageState {
    pub repo_path: PathBuf,
    pub remotes: Vec<crate::git::remote::RemoteInfo>,
    pub selected_idx: usize,
}

/// Add remote: `fields` are name and URL.
#[derive(Debug, Clone)]
pub struct GitRemoteAddState {
    pub repo_path: PathBuf,
    pub fields: FieldPair,
}

/// Clone: `fields` are URL and target folder name.
#[derive(Debug, Clone)]
pub struct GitClonePromptState {
    pub target_parent_path: PathBuf,
    pub fields: FieldPair,
}

/// Progress of the background Git network operation (`AppState::git_op`).
#[derive(Debug, Clone)]
pub struct GitProgress {
    pub title: String,
}
