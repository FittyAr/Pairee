#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    /// Name of the file currently being copied/moved
    pub current_file: String,
    /// Number of files fully copied so far
    pub files_copied: usize,
    /// Total number of files to copy
    pub total_files: usize,
    /// Total number of bytes copied so far across all files
    pub bytes_copied: u64,
    /// Total bytes to copy across all files
    pub total_bytes: u64,
    /// Detailed error message if the task fails
    pub error: Option<String>,
}

pub mod compress;
pub mod copy_move;
pub mod delete;
pub mod extract;
pub mod helper;
pub mod wipe;

pub use compress::spawn_compress_task;
pub use copy_move::spawn_copy_move_task;
pub use delete::spawn_ssh_delete_task;
pub use extract::spawn_extract_task;
pub use wipe::spawn_wipe_task;
