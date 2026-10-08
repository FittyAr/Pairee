use crate::app::form::{FieldPair, FormLayout};
use crate::app::text_input::TextField;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct GitCommitPromptState {
    pub input: TextField,
    pub repo_path: PathBuf,
    pub is_amend: bool,
    pub previous_popup: Option<Box<super::PopupType>>,
}

#[derive(Debug, Clone)]
pub struct GitConfirmCheckoutState {
    pub target: String,
    pub is_branch: bool,
    pub repo_path: PathBuf,
    pub previous_popup: Option<Box<super::PopupType>>,
}

#[derive(Debug, Clone)]
pub struct GitDiffViewState {
    pub repo_path: PathBuf,
    pub file_path: Option<String>,
    pub commit_hash: Option<String>,
    pub diff_content: String,
    pub scroll_y: usize,
    pub previous_popup: Box<super::PopupType>,
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
    pub previous_popup: Box<super::PopupType>,
}

impl GitNamePromptState {
    /// Name field, OK, Cancel.
    pub const FORM: FormLayout = FormLayout::with_input(3, 1);
    pub const BUTTON_CANCEL: usize = 2;

    /// A prompt for `action`; renaming starts from the current name.
    pub fn new(
        action: GitNameAction,
        repo_path: PathBuf,
        previous_popup: Box<super::PopupType>,
    ) -> Self {
        let input = match &action {
            GitNameAction::RenameBranch { old_name } => TextField::new(old_name.as_str()),
            _ => TextField::default(),
        };
        Self {
            action,
            input,
            cursor_idx: 0,
            repo_path,
            previous_popup,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitConfirmActionState {
    pub message: String,
    pub repo_path: PathBuf,
    pub action: crate::app::state::types::GitConfirmedAction,
    pub previous_popup: Box<super::PopupType>,
}

#[derive(Debug, Clone)]
pub struct GitRemoteManageState {
    pub repo_path: PathBuf,
    pub remotes: Vec<crate::git::remote::RemoteInfo>,
    pub selected_idx: usize,
    pub previous_popup: Box<super::PopupType>,
}

/// Add remote: `fields` are name and URL.
#[derive(Debug, Clone)]
pub struct GitRemoteAddState {
    pub repo_path: PathBuf,
    pub fields: FieldPair,
    pub previous_popup: Box<super::PopupType>,
}

/// Clone: `fields` are URL and target folder name.
#[derive(Debug, Clone)]
pub struct GitClonePromptState {
    pub target_parent_path: PathBuf,
    pub fields: FieldPair,
}

#[derive(Debug, Clone)]
pub enum GitPromptPopup {
    CommitPrompt(GitCommitPromptState),
    ConfirmCheckout(GitConfirmCheckoutState),
    DiffView(GitDiffViewState),
    NamePrompt(GitNamePromptState),
    ConfirmAction(GitConfirmActionState),
    RemoteManage(GitRemoteManageState),
    RemoteAdd(GitRemoteAddState),
    ClonePrompt(GitClonePromptState),
}
