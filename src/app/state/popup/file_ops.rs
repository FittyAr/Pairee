use crate::app::text_input::TextField;
use crate::fs::transfer::job::TransferOperation;
use crate::fs::transfer::options::TransferOptions;
use std::path::{Path, PathBuf};

/// Which transfer the Copy / Move dialog configures (Strategy for the shared
/// transfer dialog: labels, job kind and tree-view caller differ, nothing else).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferPromptOp {
    Copy,
    Move,
}

impl TransferPromptOp {
    /// The transfer job this dialog submits.
    pub fn operation(self) -> TransferOperation {
        match self {
            Self::Copy => TransferOperation::Copy,
            Self::Move => TransferOperation::Move,
        }
    }

    /// Localization keys used by the dialog renderer.
    pub fn labels(self) -> TransferPromptLabels {
        match self {
            Self::Copy => TransferPromptLabels {
                title: "prompt_copy_title",
                single: "prompt_copy_sing",
                plural: "prompt_copy_plur",
                to: "prompt_copy_to",
                button: "btn_copy_bracket",
            },
            Self::Move => TransferPromptLabels {
                title: "prompt_move_title",
                single: "prompt_move_sing",
                plural: "prompt_move_plur",
                to: "prompt_move_to",
                button: "btn_move_bracket",
            },
        }
    }
}

/// Localization keys of one [`TransferPromptOp`].
#[derive(Debug, Clone, Copy)]
pub struct TransferPromptLabels {
    pub title: &'static str,
    pub single: &'static str,
    pub plural: &'static str,
    pub to: &'static str,
    pub button: &'static str,
}

/// State of the Copy (F5) / Move (F6) dialog.
#[derive(Debug, Clone)]
pub struct CopyMovePromptState {
    pub op: TransferPromptOp,
    pub input: TextField,
    pub src_paths: Vec<PathBuf>,
    pub dest_dir: PathBuf,
    pub cursor_idx: usize,
    pub already_existing: usize,
    pub process_multiple: bool,
    pub copy_access_mode: bool,
    pub copy_extended_attributes: bool,
    pub disable_write_cache: bool,
    pub produce_sparse_files: bool,
    pub use_copy_on_write: bool,
    pub symlink_mode: usize,
    pub use_filter: bool,
    pub filter_mask: String,
}

impl CopyMovePromptState {
    /// Dialog for `src_paths` into `dest_dir`, with default options. A single
    /// source pre-fills `dest_dir/<name>`, several sources just `dest_dir`.
    pub fn new(op: TransferPromptOp, src_paths: Vec<PathBuf>, dest_dir: PathBuf) -> Self {
        let input = default_destination(&src_paths, &dest_dir);
        Self {
            op,
            input: TextField::new(input),
            src_paths,
            dest_dir,
            cursor_idx: 0,
            already_existing: 0, // Ask
            process_multiple: false,
            copy_access_mode: true,
            copy_extended_attributes: false,
            disable_write_cache: false,
            produce_sparse_files: false,
            use_copy_on_write: false,
            symlink_mode: 0,
            use_filter: false,
            filter_mask: String::new(),
        }
    }

    /// Destination typed by the user: empty means `dest_dir`, a relative path
    /// is resolved against `dest_dir`.
    pub fn destination(&self) -> PathBuf {
        let input = self.input.text();
        if input.trim().is_empty() {
            self.dest_dir.clone()
        } else {
            self.dest_dir.join(input)
        }
    }
}

/// Focusable rows of the transfer dialog (`cursor_idx` values).
impl CopyMovePromptState {
    pub const ROW_INPUT: usize = 0;
    pub const ROW_EXISTING: usize = 1;
    /// First of the six checkbox rows (`2..=7`, see [`Self::flags`]).
    pub const ROW_FIRST_FLAG: usize = 2;
    pub const ROW_SYMLINKS: usize = 8;
    pub const ROW_FILTER: usize = 9;
    pub const BUTTON_SUBMIT: usize = 10;
    pub const BUTTON_TREE: usize = 11;
    pub const BUTTON_FILTER: usize = 12;
    pub const BUTTON_CANCEL: usize = 13;
    pub const ROW_COUNT: usize = 14;
    /// Choices of the "already existing files" row.
    pub const EXISTING_CHOICES: usize = 4;
    /// Choices of the symbolic-links row.
    pub const SYMLINK_CHOICES: usize = 3;

    /// The checkbox rows in display order, with their localization keys.
    pub fn flags(&self) -> [(bool, &'static str); 6] {
        [
            (self.process_multiple, "prompt_process_multiple"),
            (self.copy_access_mode, "prompt_copy_files_access"),
            (self.copy_extended_attributes, "prompt_copy_ext_attr"),
            (self.disable_write_cache, "prompt_disable_write_cache"),
            (self.produce_sparse_files, "prompt_produce_sparse_files"),
            (self.use_copy_on_write, "prompt_use_cow"),
        ]
    }

    /// `true` for the button bar rows.
    pub fn is_button(row: usize) -> bool {
        (Self::BUTTON_SUBMIT..=Self::BUTTON_CANCEL).contains(&row)
    }

    /// Space on an option row: flips a checkbox or cycles a choice.
    pub fn toggle(&mut self, row: usize) {
        match row {
            Self::ROW_EXISTING => {
                self.already_existing = (self.already_existing + 1) % Self::EXISTING_CHOICES;
            }
            Self::ROW_SYMLINKS => {
                self.symlink_mode = (self.symlink_mode + 1) % Self::SYMLINK_CHOICES;
            }
            _ => {
                if let Some(flag) = self.flag_mut(row) {
                    *flag = !*flag;
                }
            }
        }
    }

    fn flag_mut(&mut self, row: usize) -> Option<&mut bool> {
        Some(match row {
            2 => &mut self.process_multiple,
            3 => &mut self.copy_access_mode,
            4 => &mut self.copy_extended_attributes,
            5 => &mut self.disable_write_cache,
            6 => &mut self.produce_sparse_files,
            7 => &mut self.use_copy_on_write,
            Self::ROW_FILTER => &mut self.use_filter,
            _ => return None,
        })
    }

    /// Applies the dialog choices on top of the settings-derived `options`.
    pub fn apply_to(&self, options: &mut TransferOptions) {
        options.direct_io = self.disable_write_cache;
        options.preserve_attributes = self.copy_extended_attributes;
        options.conflict_resolution = match self.already_existing {
            1 => "overwrite",
            2 => "skip",
            3 => "overwrite_older",
            4 => "rename",
            _ => "ask",
        }
        .to_string();
        (options.skip_symlinks, options.follow_symlinks) = match self.symlink_mode {
            1 => (false, true),
            2 => (true, false),
            _ => (false, false),
        };
        options.filter_mask =
            (self.use_filter && !self.filter_mask.is_empty()).then(|| self.filter_mask.clone());
    }
}

fn default_destination(src_paths: &[PathBuf], dest_dir: &Path) -> String {
    match src_paths {
        [single] => single
            .file_name()
            .map(|n| dest_dir.join(n))
            .unwrap_or_else(|| dest_dir.to_path_buf()),
        _ => dest_dir.to_path_buf(),
    }
    .to_string_lossy()
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_source_prefills_its_name() {
        let p = CopyMovePromptState::new(
            TransferPromptOp::Copy,
            vec![PathBuf::from("src").join("a.txt")],
            PathBuf::from("dst"),
        );
        assert_eq!(
            p.input.text(),
            PathBuf::from("dst").join("a.txt").to_string_lossy()
        );
    }

    #[test]
    fn destination_resolves_relative_and_blank_input() {
        let mut p = CopyMovePromptState::new(
            TransferPromptOp::Move,
            vec![PathBuf::from("a"), PathBuf::from("b")],
            PathBuf::from("dst"),
        );
        assert_eq!(p.input.text(), "dst");
        p.input.set_text("  ");
        assert_eq!(p.destination(), PathBuf::from("dst"));
        p.input.set_text("sub");
        assert_eq!(p.destination(), PathBuf::from("dst").join("sub"));
    }

    #[test]
    fn toggles_and_options_mapping() {
        let mut p = CopyMovePromptState::new(TransferPromptOp::Copy, vec![], PathBuf::from("d"));
        for _ in 0..3 {
            p.toggle(CopyMovePromptState::ROW_EXISTING);
        }
        p.toggle(CopyMovePromptState::ROW_SYMLINKS);
        p.toggle(4);
        p.toggle(CopyMovePromptState::ROW_FILTER);
        p.filter_mask = "*.rs".into();
        let mut options = TransferOptions::default();
        p.apply_to(&mut options);
        assert_eq!(options.conflict_resolution, "overwrite_older");
        assert!(options.follow_symlinks && !options.skip_symlinks);
        assert!(options.preserve_attributes);
        assert_eq!(options.filter_mask.as_deref(), Some("*.rs"));
        p.toggle(CopyMovePromptState::ROW_EXISTING);
        assert_eq!(p.already_existing, 0);
    }
}
