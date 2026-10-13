//! Prompts of the file operations: create, rename, compress, apply a
//! command, describe, select a group, link, the Copy / Move file mask, and
//! the delete / wipe confirmations.

use super::CreateKind;
use crate::app::state::types::{LinkKind, SelectMode};
use crate::app::text_input::TextField;
use std::path::PathBuf;

/// Make folder / new file (F7, `a`).
#[derive(Debug, Clone)]
pub struct MkDirPrompt {
    pub input: TextField,
    pub cursor_idx: usize,
    pub process_multiple: bool,
    /// Folder (`mkdir`), file (`new_file`) or either (`create`).
    pub kind: CreateKind,
}

/// Rename (Shift+F6 on one entry).
#[derive(Debug, Clone)]
pub struct RenamePrompt {
    pub input: TextField,
    pub original: String,
    pub src_path: PathBuf,
    pub parent_dir: PathBuf,
    pub cursor_idx: usize,
}

/// Name of the zip to pack the selection into.
#[derive(Debug, Clone)]
pub struct CompressPrompt {
    pub input: TextField,
    pub targets: Vec<PathBuf>,
    pub dest_dir: PathBuf,
}

/// Command to run on each selected file.
#[derive(Debug, Clone)]
pub struct ApplyCommandPrompt {
    pub input: TextField,
    pub targets: Vec<PathBuf>,
}

/// Description of a file (`descript.ion`).
#[derive(Debug, Clone)]
pub struct DescribeFilePrompt {
    pub path: PathBuf,
    pub current_desc: String,
    pub input: TextField,
}

/// Mask to select (`+`) or deselect (`-`) entries.
#[derive(Debug, Clone)]
pub struct SelectGroupPrompt {
    pub mode: SelectMode,
    pub query: TextField,
}

/// Destination of a symbolic or hard link.
#[derive(Debug, Clone)]
pub struct CreateLinkPrompt {
    pub src: PathBuf,
    pub dest_input: TextField,
    pub kind: LinkKind,
}

/// File mask of the Copy / Move dialog, opened over it.
#[derive(Debug, Clone)]
pub struct CopyMoveFilterPrompt {
    pub input: TextField,
}

/// Delete (F8) confirmation with [Delete] / [Cancel] buttons.
#[derive(Debug, Clone)]
pub struct ConfirmDelete {
    pub paths: Vec<PathBuf>,
    pub cursor_idx: usize,
    /// Goes to the recycle bin (else deleted for good).
    pub to_trash: bool,
}

/// Wipe (overwrite, then delete) confirmation.
#[derive(Debug, Clone)]
pub struct WipeConfirm {
    pub paths: Vec<PathBuf>,
}
