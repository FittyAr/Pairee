use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

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

/// Spawns a background task to copy multiple source files/directories to a destination directory.
/// Returns a channel receiver for real-time progress updates.
pub fn spawn_copy_task(
    sources: Vec<PathBuf>,
    destination_dir: PathBuf,
) -> mpsc::Receiver<ProgressUpdate> {
    let (tx, rx) = mpsc::channel(100);

    tokio::spawn(async move {
        let mut total_files = 0;
        let mut total_bytes = 0;
        let mut file_mappings = Vec::new(); // Pair of (src, dst)
        let mut dirs_to_create = Vec::new();

        // 1. Gather all directories to create and files to copy
        for src in &sources {
            if src.is_dir() {
                if let Some(folder_name) = src.file_name() {
                    let base_dst = destination_dir.join(folder_name);
                    dirs_to_create.push(base_dst.clone());

                    let mut dirs_to_visit = vec![src.clone()];
                    while let Some(dir) = dirs_to_visit.pop() {
                        if let Ok(entries) = fs::read_dir(&dir) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if path.is_dir() {
                                    dirs_to_visit.push(path.clone());
                                    if let Ok(rel) = path.strip_prefix(src) {
                                        let dst_dir = base_dst.join(rel);
                                        dirs_to_create.push(dst_dir);
                                    }
                                } else {
                                    total_files += 1;
                                    if let Ok(meta) = entry.metadata() {
                                        total_bytes += meta.len();
                                    }
                                    if let Ok(rel) = path.strip_prefix(src) {
                                        let dst_path = base_dst.join(rel);
                                        file_mappings.push((path, dst_path));
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                total_files += 1;
                if let Ok(meta) = src.metadata() {
                    total_bytes += meta.len();
                }
                if let Some(file_name) = src.file_name() {
                    let dst_path = destination_dir.join(file_name);
                    file_mappings.push((src.clone(), dst_path));
                }
            }
        }

        // 2. Create the target folder structures
        for dir in dirs_to_create {
            if let Err(e) = fs::create_dir_all(&dir) {
                let _ = tx
                    .send(ProgressUpdate {
                        current_file: dir.to_string_lossy().into_owned(),
                        files_copied: 0,
                        total_files,
                        bytes_copied: 0,
                        total_bytes,
                        error: Some(format!("Failed to create folder {:?}: {}", dir, e)),
                    })
                    .await;
                return;
            }
        }

        // 3. Copy files block by block
        let mut files_copied = 0;
        let mut bytes_copied = 0;

        // In case there were only empty folders, trigger a finish
        if file_mappings.is_empty() {
            let _ = tx
                .send(ProgressUpdate {
                    current_file: "Completed".to_string(),
                    files_copied: total_files,
                    total_files,
                    bytes_copied: total_bytes,
                    total_bytes,
                    error: None,
                })
                .await;
            return;
        }

        for (src, dst) in file_mappings {
            let file_name = src
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();

            // Send starting file notification
            let _ = tx
                .send(ProgressUpdate {
                    current_file: file_name.clone(),
                    files_copied,
                    total_files,
                    bytes_copied,
                    total_bytes,
                    error: None,
                })
                .await;

            if let Some(parent) = dst.parent() {
                let _ = fs::create_dir_all(parent);
            }

            match copy_file_buffered(
                &src,
                &dst,
                &tx,
                &mut bytes_copied,
                &file_name,
                files_copied,
                total_files,
                total_bytes,
            )
            .await
            {
                Ok(_) => {
                    files_copied += 1;
                }
                Err(e) => {
                    let _ = tx
                        .send(ProgressUpdate {
                            current_file: file_name,
                            files_copied,
                            total_files,
                            bytes_copied,
                            total_bytes,
                            error: Some(format!("Error copying file {:?}: {}", src, e)),
                        })
                        .await;
                    return;
                }
            }
        }

        // 4. Send final completion update
        let _ = tx
            .send(ProgressUpdate {
                current_file: "Completed".to_string(),
                files_copied,
                total_files,
                bytes_copied,
                total_bytes,
                error: None,
            })
            .await;
    });

    rx
}

/// Copies a single file in chunks to allow cancellation or smooth progress updates.
#[allow(clippy::too_many_arguments)]
async fn copy_file_buffered(
    src: &Path,
    dst: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
    global_bytes_copied: &mut u64,
    file_name: &str,
    files_copied: usize,
    total_files: usize,
    total_bytes: u64,
) -> Result<()> {
    use std::io::{Read, Write};
    let mut src_file = fs::File::open(src)?;
    let mut dst_file = fs::File::create(dst)?;

    let mut buffer = vec![0; 64 * 1024]; // 64 KB buffer size
    loop {
        let bytes_read = src_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        dst_file.write_all(&buffer[..bytes_read])?;
        *global_bytes_copied += bytes_read as u64;

        // Stream current status update
        let _ = tx
            .send(ProgressUpdate {
                current_file: file_name.to_string(),
                files_copied,
                total_files,
                bytes_copied: *global_bytes_copied,
                total_bytes,
                error: None,
            })
            .await;
    }
    Ok(())
}
