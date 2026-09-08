use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CopyMovePromptState {
    pub input: String,
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
