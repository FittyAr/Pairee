//! Shared fixtures for archive tests: building archives in memory/on disk,
//! running an extraction without a UI, and snapshotting extracted trees.

use crate::fs::progress::ProgressUpdate;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;

/// Large enough that `blocking_send` never waits on an undrained receiver.
const PROGRESS_CAPACITY: usize = 1024;

/// Files used by round-trip tests: `(relative path, contents)`.
pub const SAMPLE_TREE: &[(&str, &[u8])] = &[
    ("a.txt", b"alpha"),
    ("sub/b.txt", b"bravo"),
    ("sub/deeper/c.bin", &[0, 1, 2, 3, 255]),
];

pub fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    for (name, body) in entries {
        zip.start_file(*name, opts).unwrap();
        zip.write_all(body).unwrap();
    }
    zip.finish().unwrap();
}

pub fn write_tar_gz(path: &Path, entries: &[(&str, &[u8])]) {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, body) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, name, *body).unwrap();
    }
    let tar_bytes = builder.into_inner().unwrap();
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gz.write_all(&tar_bytes).unwrap();
    fs::write(path, gz.finish().unwrap()).unwrap();
}

/// Materialises `entries` below `root`, creating parent folders.
pub fn write_tree(root: &Path, entries: &[(&str, &[u8])]) {
    for (rel, body) in entries {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }
}

/// Every regular file below `root`, keyed by `/`-separated relative path.
pub fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path.strip_prefix(root).unwrap();
                let key = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.insert(key, fs::read(&path).unwrap());
            }
        }
    }
    out
}

/// The expected [`snapshot`] of `entries`, optionally below `prefix/`.
pub fn expected(entries: &[(&str, &[u8])], prefix: Option<&str>) -> BTreeMap<String, Vec<u8>> {
    entries
        .iter()
        .map(|(rel, body)| {
            let key = match prefix {
                Some(p) => format!("{p}/{rel}"),
                None => rel.to_string(),
            };
            (key, body.to_vec())
        })
        .collect()
}

/// Runs an extract/compress function with a fresh progress channel and no
/// cancellation.
pub fn run<T>(
    op: impl FnOnce(&mpsc::Sender<ProgressUpdate>, &AtomicBool) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let (tx, _rx) = mpsc::channel(PROGRESS_CAPACITY);
    op(&tx, &AtomicBool::new(false))
}
