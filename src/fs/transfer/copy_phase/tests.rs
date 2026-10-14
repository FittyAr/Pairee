//! The stream path (used for SFTP) exercised on local folders: the same
//! adapters' primitives (`create_file`, `replace`, `set_metadata`) without a
//! server.

use super::*;
use crate::fs::transfer::job::TransferJob;
use crate::fs::transfer::part::part_path;
use crate::fs::transfer::scan::{ScanRequest, scan, top_targets};
use crate::fs::vfs::LocalVfs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Local folders reached through the stream copy, as a remote job would.
fn stream_endpoints() -> Endpoints {
    Endpoints {
        local: false,
        ..Endpoints::local()
    }
}

async fn run_stream(
    operation: TransferOperation,
    sources: &[PathBuf],
    destination: &Path,
    options: TransferOptions,
) -> TransferResults {
    run_with(stream_endpoints(), operation, sources, destination, options).await
}

async fn run_with(
    endpoints: Endpoints,
    operation: TransferOperation,
    sources: &[PathBuf],
    destination: &Path,
    options: TransferOptions,
) -> TransferResults {
    let (tx, mut rx) = crate::fs::transfer::events::EventSender::channel();
    tokio::spawn(async move { while rx.recv().await.is_some() {} });
    let job = TransferJob::new(
        operation,
        sources.to_vec(),
        destination.to_path_buf(),
        options,
    );
    let ctl = JobControl::for_job(&job, tx);
    let roots = top_targets(
        sources,
        destination,
        operation,
        Default::default(),
        endpoints.dst.as_ref(),
    );
    let request = ScanRequest {
        roots: &roots,
        operation,
        options: &job.options,
        src_fs: endpoints.src.as_ref(),
        dst_fs: endpoints.dst.as_ref(),
        same_fs: true,
        dst_links: false,
    };
    let scan = scan(&request, &ctl).unwrap();
    let request = CopyRequest {
        operation,
        scan,
        options: &job.options,
        endpoints,
        slot: job.active_conflict.clone(),
    };
    run_copy_phase(request, &ctl, TransferResults::default())
        .await
        .unwrap()
}

/// FS-09: SSH copies honour the conflict setting, verify, keep times and
/// replace atomically.
#[tokio::test]
async fn stream_copies_resolve_conflicts_verify_and_keep_times() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(src.join("sub")).unwrap();
    std::fs::write(src.join("new.txt"), b"new").unwrap();
    std::fs::write(src.join("sub").join("old.txt"), b"source").unwrap();
    let when = SystemTime::UNIX_EPOCH + Duration::from_secs(1_500_000_000);
    filetime::set_file_mtime(
        src.join("new.txt"),
        filetime::FileTime::from_system_time(when),
    )
    .unwrap();
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(dst.join("src").join("sub")).unwrap();
    std::fs::write(dst.join("src").join("sub").join("old.txt"), b"keep").unwrap();

    let options = TransferOptions {
        conflict_resolution: "skip".into(),
        verify_after_copy: true,
        ..TransferOptions::default()
    };
    let results = run_stream(
        TransferOperation::Copy,
        std::slice::from_ref(&src),
        &dst,
        options,
    )
    .await;
    assert_eq!(results.completed_files.len(), 1);
    assert!(results.completed_files[0].verified);
    assert_eq!(results.skipped_files.len(), 1);
    let copied = dst.join("src").join("new.txt");
    assert_eq!(std::fs::read(&copied).unwrap(), b"new");
    assert_eq!(
        std::fs::metadata(&copied).unwrap().modified().unwrap(),
        when
    );
    assert!(!part_path(&copied).exists());
    assert_eq!(
        std::fs::read(dst.join("src").join("sub").join("old.txt")).unwrap(),
        b"keep"
    );
}

/// FS-09 / FS-12: a stream move deletes each source only after its copy,
/// and the emptied folders afterwards.
#[tokio::test]
async fn stream_moves_remove_sources_after_copying() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("tree");
    std::fs::create_dir_all(src.join("sub")).unwrap();
    std::fs::write(src.join("sub").join("f.txt"), b"f").unwrap();
    let dst = dir.path().join("out");
    std::fs::create_dir(&dst).unwrap();
    let results = run_stream(
        TransferOperation::Move,
        std::slice::from_ref(&src),
        &dst,
        TransferOptions::default(),
    )
    .await;
    assert_eq!(results.completed_files.len(), 1);
    assert!(!src.exists());
    assert_eq!(
        std::fs::read(dst.join("tree").join("sub").join("f.txt")).unwrap(),
        b"f"
    );
}

/// A local folder seen through an adapter that records how the part file
/// of a copy was opened (resume over SFTP without a server).
#[derive(Debug, Default)]
struct Recorder {
    appended: std::sync::Mutex<Vec<PathBuf>>,
}

impl Vfs for Recorder {
    fn capabilities(&self) -> crate::fs::vfs::Capabilities {
        LocalVfs.capabilities()
    }
    fn list(&self, dir: &Path) -> std::io::Result<Vec<crate::fs::vfs::VfsEntry>> {
        LocalVfs.list(dir)
    }
    fn stat(&self, path: &Path) -> std::io::Result<crate::fs::vfs::VfsEntry> {
        LocalVfs.stat(path)
    }
    fn open_read(&self, path: &Path) -> std::io::Result<Box<dyn std::io::Read + Send>> {
        LocalVfs.open_read(path)
    }
    fn create_file(&self, path: &Path) -> std::io::Result<Box<dyn std::io::Write + Send>> {
        LocalVfs.create_file(path)
    }
    fn append_file(&self, path: &Path) -> std::io::Result<Box<dyn std::io::Write + Send>> {
        self.appended.lock().unwrap().push(path.to_path_buf());
        LocalVfs.append_file(path)
    }
    fn replace(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        LocalVfs.replace(from, to)
    }
    fn mkdir(&self, path: &Path) -> std::io::Result<()> {
        LocalVfs.mkdir(path)
    }
    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        LocalVfs.remove_file(path)
    }
}

/// Copies `data` into an empty folder whose part file already holds
/// `left`; returns the copy and the part files that were appended to.
async fn resume_with(data: &[u8], left: &[u8]) -> (Vec<u8>, Vec<PathBuf>) {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("big.bin");
    std::fs::write(&src, data).unwrap();
    let dst = dir.path().join("out");
    std::fs::create_dir(&dst).unwrap();
    let copied = dst.join("big.bin");
    std::fs::write(part_path(&copied), left).unwrap();
    let recorder = Arc::new(Recorder::default());
    let endpoints = Endpoints {
        dst: recorder.clone(),
        same_fs: false,
        ..stream_endpoints()
    };
    let options = TransferOptions {
        verify_after_copy: true,
        ..TransferOptions::default()
    };
    let results = run_with(
        endpoints,
        TransferOperation::Copy,
        std::slice::from_ref(&src),
        &dst,
        options,
    )
    .await;
    assert_eq!(results.completed_files.len(), 1);
    assert!(results.completed_files[0].verified);
    assert!(!part_path(&copied).exists());
    let appended = recorder.appended.lock().unwrap().clone();
    (std::fs::read(&copied).unwrap(), appended)
}

/// A remote copy resumes an interrupted part file whose bytes match the
/// source, and starts over when they do not.
#[tokio::test]
async fn stream_copies_resume_matching_part_files() {
    let data: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
    let (copy, appended) = resume_with(&data, &data[..40_000]).await;
    assert_eq!(copy, data);
    assert_eq!(appended.len(), 1);

    let (copy, appended) = resume_with(&data, b"not the source").await;
    assert_eq!(copy, data);
    assert!(appended.is_empty());
}

/// Uploads and downloads through a real SSH server resume the part files
/// an interrupted copy left (opt-in, see `fs::ssh::server_tests`).
#[cfg(pairee_sftp_test)]
#[tokio::test(flavor = "multi_thread")]
async fn sftp_server_copies_resume_part_files_both_ways() {
    use crate::fs::ssh::server_tests::ServerFolder;
    let data: Vec<u8> = (0..3_000_000u32).map(|i| (i % 249) as u8).collect();
    let folder = ServerFolder::new();
    let local = tempfile::tempdir().unwrap();
    let src = local.path().join("big.bin");
    std::fs::write(&src, &data).unwrap();
    let options = || TransferOptions {
        verify_after_copy: true,
        ..TransferOptions::default()
    };

    // Upload over a part file holding the first million bytes.
    let remote = folder.path.join("big.bin");
    folder
        .vfs
        .write_file(&part_path(&remote), &mut &data[..1_000_000])
        .unwrap();
    let up = Endpoints {
        dst: Arc::clone(&folder.vfs),
        same_fs: false,
        ..stream_endpoints()
    };
    let results = run_with(
        up,
        TransferOperation::Copy,
        std::slice::from_ref(&src),
        &folder.path,
        options(),
    )
    .await;
    assert_eq!(
        results.completed_files.len(),
        1,
        "{:?}",
        results.failed_files
    );
    assert!(!folder.vfs.exists(&part_path(&remote)));
    let mut uploaded = Vec::new();
    std::io::Read::read_to_end(&mut folder.vfs.open_read(&remote).unwrap(), &mut uploaded).unwrap();
    assert_eq!(uploaded, data);

    // Download over a local part file.
    let back = local.path().join("back");
    std::fs::create_dir(&back).unwrap();
    std::fs::write(part_path(&back.join("big.bin")), &data[..2_000_000]).unwrap();
    let down = Endpoints {
        src: Arc::clone(&folder.vfs),
        same_fs: false,
        ..stream_endpoints()
    };
    let results = run_with(
        down,
        TransferOperation::Copy,
        std::slice::from_ref(&remote),
        &back,
        options(),
    )
    .await;
    assert_eq!(
        results.completed_files.len(),
        1,
        "{:?}",
        results.failed_files
    );
    assert_eq!(std::fs::read(back.join("big.bin")).unwrap(), data);
}

/// A destination whose lookup fails for another reason than "not found"
/// (here a timeout).
#[derive(Debug)]
struct FlakyLookup(PathBuf);

impl Vfs for FlakyLookup {
    fn capabilities(&self) -> crate::fs::vfs::Capabilities {
        LocalVfs.capabilities()
    }
    fn list(&self, dir: &Path) -> std::io::Result<Vec<crate::fs::vfs::VfsEntry>> {
        LocalVfs.list(dir)
    }
    fn stat(&self, path: &Path) -> std::io::Result<crate::fs::vfs::VfsEntry> {
        if path == self.0 {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        LocalVfs.stat(path)
    }
    fn create_file(&self, path: &Path) -> std::io::Result<Box<dyn std::io::Write + Send>> {
        LocalVfs.create_file(path)
    }
    fn replace(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        LocalVfs.replace(from, to)
    }
    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        LocalVfs.remove_file(path)
    }
}

/// A destination that cannot be looked up is a failure, never a free
/// place to overwrite without asking.
#[tokio::test]
async fn a_destination_that_cannot_be_looked_up_is_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.txt");
    std::fs::write(&src, b"new").unwrap();
    let dst = dir.path().join("out");
    std::fs::create_dir(&dst).unwrap();
    let existing = dst.join("a.txt");
    std::fs::write(&existing, b"keep").unwrap();
    let endpoints = Endpoints {
        dst: Arc::new(FlakyLookup(existing.clone())),
        same_fs: false,
        ..stream_endpoints()
    };
    let options = TransferOptions {
        conflict_resolution: "skip".into(),
        ..TransferOptions::default()
    };
    let results = run_with(
        endpoints,
        TransferOperation::Copy,
        std::slice::from_ref(&src),
        &dst,
        options,
    )
    .await;
    assert_eq!(results.failed_files.len(), 1);
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep");
}

/// A file copied onto a folder of the same name through an adapter fails
/// and leaves the folder alone (WebDAV would delete it to replace it).
#[tokio::test]
async fn a_stream_copy_never_replaces_a_folder() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("x");
    std::fs::write(&src, b"file").unwrap();
    let dst = dir.path().join("out");
    std::fs::create_dir_all(dst.join("x")).unwrap();
    std::fs::write(dst.join("x").join("keep.txt"), b"keep").unwrap();
    let options = TransferOptions {
        conflict_resolution: "overwrite".into(),
        ..TransferOptions::default()
    };
    let results = run_stream(
        TransferOperation::Copy,
        std::slice::from_ref(&src),
        &dst,
        options,
    )
    .await;
    assert_eq!(results.failed_files.len(), 1);
    assert_eq!(
        std::fs::read(dst.join("x").join("keep.txt")).unwrap(),
        b"keep"
    );
}
