use super::super::super::super::events::TransferEvent;
use super::super::super::super::worker::is_destination_parent_dir;
use crate::fs::ssh::SharedSshClient;
use crate::fs::transfer::control::JobControl;
use anyhow::anyhow;
use std::path::{Path, PathBuf};

pub struct ScanOutput {
    pub total_bytes: u64,
    pub file_mappings: Vec<(PathBuf, PathBuf, u64)>,
    pub dirs_to_create: Vec<PathBuf>,
}

pub fn scan_sources(
    sources: &[PathBuf],
    destination_dir: &Path,
    src_conn: &Option<SharedSshClient>,
    dst_conn: &Option<SharedSshClient>,
    control: &JobControl,
) -> Result<ScanOutput, anyhow::Error> {
    let mut total_files = 0usize;
    let mut total_bytes = 0u64;
    let mut file_mappings = Vec::new();
    let mut dirs_to_create = Vec::new();
    let src_fs = crate::fs::ssh::endpoint_vfs(src_conn);

    let destination_dir_is_dir = is_destination_parent_dir(sources, destination_dir, |p| {
        crate::fs::ssh::is_dir_on(p, dst_conn)
    });

    for src in sources {
        if control.is_cancelled() {
            return Err(anyhow!("Job cancelled"));
        }
        let is_dir = crate::fs::ssh::is_dir_on(src, src_conn);
        let name = src
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        let base_dst = if destination_dir_is_dir {
            destination_dir.join(&name)
        } else {
            destination_dir.to_path_buf()
        };
        if is_dir {
            dirs_to_create.push(base_dst.clone());
            // Links inside the tree are copied as what they point to only
            // when they are files; linked folders are created, not entered.
            for entry in src_fs.walk(src) {
                let Ok(rel) = entry.path.strip_prefix(src) else {
                    continue;
                };
                let sub_dst = base_dst.join(rel);
                if entry.is_dir {
                    dirs_to_create.push(sub_dst);
                } else {
                    total_files += 1;
                    total_bytes += entry.size;
                    file_mappings.push((entry.path, sub_dst, entry.size));
                }
            }
        } else {
            total_files += 1;
            let size = src_fs.stat(src).map_or(0, |e| e.size);
            total_bytes += size;
            file_mappings.push((src.clone(), base_dst, size));
        }

        let _ = control.event_tx.send(TransferEvent::ScanProgress {
            job_id: control.job_id,
            files_found: total_files,
        });
    }

    let _ = control.event_tx.send(TransferEvent::ScanComplete {
        job_id: control.job_id,
        total_files,
        total_bytes,
    });

    Ok(ScanOutput {
        total_bytes,
        file_mappings,
        dirs_to_create,
    })
}
