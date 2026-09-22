use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct GitCommitPromptState {
    pub input: String,
    pub cursor_idx: usize,
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

#[derive(Debug, Clone)]
pub struct GitBranchCreatePromptState {
    pub input: String,
    pub cursor_idx: usize,
    pub start_point: String,
    pub repo_path: PathBuf,
    pub previous_popup: Box<super::PopupType>,
}

#[derive(Debug, Clone)]
pub struct GitBranchRenamePromptState {
    pub input: String,
    pub cursor_idx: usize,
    pub old_name: String,
    pub repo_path: PathBuf,
    pub previous_popup: Box<super::PopupType>,
}

#[derive(Debug, Clone)]
pub struct GitStashSavePromptState {
    pub input: String,
    pub cursor_idx: usize,
    pub include_untracked: bool,
    pub repo_path: PathBuf,
    pub previous_popup: Box<super::PopupType>,
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

#[derive(Debug, Clone)]
pub struct GitRemoteAddState {
    pub repo_path: PathBuf,
    pub name_input: String,
    pub url_input: String,
    pub focus_url: bool,
    pub name_cursor: usize,
    pub url_cursor: usize,
    pub previous_popup: Box<super::PopupType>,
}

#[derive(Debug, Clone)]
pub struct GitTagCreatePromptState {
    pub input: String,
    pub cursor_idx: usize,
    pub target: String,
    pub repo_path: PathBuf,
    pub previous_popup: Box<super::PopupType>,
}

#[derive(Debug, Clone)]
pub enum GitPromptPopup {
    CommitPrompt(GitCommitPromptState),
    ConfirmCheckout(GitConfirmCheckoutState),
    DiffView(GitDiffViewState),
    BranchCreatePrompt(GitBranchCreatePromptState),
    BranchRenamePrompt(GitBranchRenamePromptState),
    StashSavePrompt(GitStashSavePromptState),
    ConfirmAction(GitConfirmActionState),
    RemoteManage(GitRemoteManageState),
    RemoteAdd(GitRemoteAddState),
    TagCreatePrompt(GitTagCreatePromptState),
}
