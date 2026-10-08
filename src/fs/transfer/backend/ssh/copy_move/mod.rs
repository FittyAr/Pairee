//! SSH copy and move file-by-file pipeline.

mod scan;

use super::super::super::events::TransferEvent;
use super::super::super::job::{FailedFile, SshEndpoints, TransferResults};
use super::fast_remote_rename;
use crate::config::localization::t;
use crate::fs::delete_util::delete_recursive;
use crate::fs::transfer::control::JobControl;
use crate::fs::transfer::control::{done, failed};
use anyhow::anyhow;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Instant;

pub fn run_ssh_copy_move(
    sources: Vec<PathBuf>,
    destination_dir: PathBuf,
    ssh: SshEndpoints,
    is_move: bool,
    control: JobControl,
) -> Result<TransferResults, anyhow::Error> {
    let src_conn = ssh.src;
    let dst_conn = ssh.dst;

    if src_conn.is_none() && dst_conn.is_none() {
        return Err(anyhow!("SSH transfer requires at least one connection"));
    }

    // Same-server fast move via SFTP rename
    if is_move
        && let (Some(src_client), Some(dst_client)) = (&src_conn, &dst_conn)
        && src_client.is_same_server(dst_client)
    {
        return fast_remote_rename(sources, destination_dir, src_client, &dst_conn, control);
    }

    let scan::ScanOutput {
        total_bytes,
        file_mappings,
        dirs_to_create,
    } = scan::scan_sources(&sources, &destination_dir, &src_conn, &dst_conn, &control)?;

    for dir in &dirs_to_create {
        if let Some(dst_client) = &dst_conn {
            let mut current = PathBuf::new();
            for component in dir.components() {
                current.push(component);
                let _ = dst_client.create_dir(&current);
            }
        } else {
            let _ = std::fs::create_dir_all(dir);
        }
    }

    let mut results = TransferResults::default();
    let mut bytes_copied_acc = 0u64;

    if file_mappings.is_empty() {
        return control.job_completed(results);
    }

    for (idx, (src, dst, size)) in file_mappings.iter().enumerate() {
        control.wait_if_paused_blocking()?;
        let start = Instant::now();
        control.file_started(src, idx);

        let copy_res = (|| -> anyhow::Result<()> {
            let mut reader: Box<dyn Read + Send> = if let Some(src_conn) = &src_conn {
                let file = src_conn.lock().sftp.open(src)?;
                Box::new(file)
            } else {
                Box::new(std::fs::File::open(src)?)
            };

            let mut writer: Box<dyn Write + Send> = if let Some(dst_conn) = &dst_conn {
                let file = dst_conn.lock().sftp.create(dst)?;
                Box::new(file)
            } else {
                if let Some(parent) = dst.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                Box::new(std::fs::File::create(dst)?)
            };

            let mut buffer = vec![0u8; 64 * 1024];
            let mut file_bytes = 0u64;
            loop {
                if control.is_cancelled() {
                    return Err(anyhow!("Job cancelled"));
                }
                let n = reader.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                writer.write_all(&buffer[..n])?;
                file_bytes += n as u64;
                let _ = control.event_tx.send(TransferEvent::FileProgress {
                    job_id: control.job_id,
                    bytes_copied: bytes_copied_acc + file_bytes,
                    bytes_total: total_bytes,
                });
            }
            bytes_copied_acc += file_bytes;
            Ok(())
        })();

        match copy_res {
            Ok(()) => control.file_completed(&mut results, done(src, dst.clone(), *size, start)),
            Err(e) => {
                let msg = t("error_copying_to")
                    .replacen("{}", &src.to_string_lossy(), 1)
                    .replacen("{}", &dst.to_string_lossy(), 1)
                    .replacen("{}", &e.to_string(), 1);
                return control.fail_job(&mut results, failed(src, dst.clone(), msg));
            }
        }
    }

    if is_move {
        for src in &sources {
            if control.is_cancelled() {
                break;
            }
            if let Some(src_client) = &src_conn {
                if let Err(e) = src_client.delete_recursive(src) {
                    let msg = t("error_remote_source_delete_failed")
                        .replacen("{}", &src.to_string_lossy(), 1)
                        .replacen("{}", &e.to_string(), 1);
                    results.failed_files.push(FailedFile {
                        src: src.clone(),
                        dst: PathBuf::new(),
                        error: msg.clone(),
                        retries: 0,
                    });
                    return Err(anyhow!(msg));
                }
            } else if let Err(e) = delete_recursive(src) {
                let msg = t("error_delete_source_failed")
                    .replacen("{}", &src.to_string_lossy(), 1)
                    .replacen("{}", &e.to_string(), 1);
                results.failed_files.push(FailedFile {
                    src: src.clone(),
                    dst: PathBuf::new(),
                    error: msg.clone(),
                    retries: 0,
                });
                return Err(anyhow!(msg));
            }
        }
    }

    control.job_completed(results)
}
