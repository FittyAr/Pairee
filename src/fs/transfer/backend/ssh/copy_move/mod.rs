//! SSH copy and move file-by-file pipeline.

mod scan;

use super::super::super::events::TransferEvent;
use super::super::super::job::{FailedFile, SshEndpoints, TransferResults};
use super::fast_remote_rename;
use crate::config::localization::t;
use crate::fs::delete_util::delete_recursive;
use crate::fs::ssh::endpoint_vfs;
use crate::fs::transfer::control::JobControl;
use crate::fs::transfer::control::{done, failed};
use crate::fs::vfs::Vfs;
use anyhow::anyhow;
use std::io::Read;
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

    let src_fs = endpoint_vfs(&src_conn);
    let dst_fs = endpoint_vfs(&dst_conn);
    for dir in &dirs_to_create {
        dst_fs.mkdir_all(dir);
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

        let copy_res = (|| -> std::io::Result<()> {
            if let Some(parent) = dst.parent() {
                dst_fs.mkdir_all(parent);
            }
            let mut reader = ProgressRead {
                inner: src_fs.open_read(src)?,
                control: &control,
                done: bytes_copied_acc,
                total: total_bytes,
            };
            dst_fs.write_file(dst, &mut reader)?;
            bytes_copied_acc = reader.done;
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
            let removed = match &src_conn {
                Some(client) => client.remove_all(src).map_err(anyhow::Error::from),
                None => delete_recursive(src),
            };
            if let Err(e) = removed {
                let key = if src_conn.is_some() {
                    "error_remote_source_delete_failed"
                } else {
                    "error_delete_source_failed"
                };
                let msg = t(key).replacen("{}", &src.to_string_lossy(), 1).replacen(
                    "{}",
                    &e.to_string(),
                    1,
                );
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

/// Reads a source file for the copy loop, reporting byte progress and
/// stopping (with an error) once the job is cancelled.
struct ProgressRead<'a> {
    inner: Box<dyn Read + Send>,
    control: &'a JobControl,
    /// Bytes of the whole job copied so far.
    done: u64,
    total: u64,
}

impl Read for ProgressRead<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.control.is_cancelled() {
            return Err(std::io::Error::other("Job cancelled"));
        }
        let n = self.inner.read(buf)?;
        self.done += n as u64;
        let _ = self.control.event_tx.send(TransferEvent::FileProgress {
            job_id: self.control.job_id,
            bytes_copied: self.done,
            bytes_total: self.total,
        });
        Ok(n)
    }
}
