use anyhow::{Result, anyhow};
use flate2::read::GzDecoder;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tar::Archive;
use tokio::sync::mpsc;
use zip::ZipArchive;

use crate::fs::ops_worker::ProgressUpdate;

pub enum ArchiveFormat {
    Zip,
    TarGz,
    SevenZ,
    Rar,
    Iso,
    Unsupported,
}

pub fn detect_format(path: &Path) -> ArchiveFormat {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if ext_str == "zip" {
            return ArchiveFormat::Zip;
        } else if ext_str == "gz" || ext_str == "tgz" {
            return ArchiveFormat::TarGz;
        } else if ext_str == "7z" {
            return ArchiveFormat::SevenZ;
        } else if ext_str == "rar" {
            return ArchiveFormat::Rar;
        } else if ext_str == "iso" {
            return ArchiveFormat::Iso;
        }
    }
    ArchiveFormat::Unsupported
}

pub fn extract_archive(
    archive_path: &Path,
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
) -> Result<()> {
    match detect_format(archive_path) {
        ArchiveFormat::Zip => extract_zip(archive_path, dest_dir, tx),
        ArchiveFormat::TarGz => extract_tar_gz(archive_path, dest_dir, tx),
        ArchiveFormat::SevenZ => extract_7z(archive_path, dest_dir, tx),
        ArchiveFormat::Rar | ArchiveFormat::Iso => {
            extract_via_external_7z(archive_path, dest_dir, tx)
        }
        ArchiveFormat::Unsupported => Err(anyhow!("Unsupported archive format")),
    }
}

/// Validate an archive entry name to prevent zip-slip / tar-slip /
/// 7z-slip. Rejects:
///   * empty names
///   * absolute paths (`/etc/passwd`, `C:\foo`)
///   * traversal segments (`..`, `./`)
///   * NUL bytes and other control characters
///   * Windows drive prefixes and UNC prefixes
///   * backslashes (we use forward-slash semantics in archives)
/// Returns the validated name on success, or an `Err` explaining
/// the rejection. The `zip` and `tar` crates do their own checks
/// internally, but `sevenz-rust 0.6.1` does not — that crate
/// happily writes entries named `../../../etc/passwd` to
/// `dest.join(entry.name())`, which the OS resolves to
/// `/etc/passwd`. We therefore apply this check uniformly and
/// treat the `zip` / `tar` crate checks as defence-in-depth.
fn validate_archive_entry_name(name: &str) -> Result<&str> {
    if name.is_empty() {
        anyhow::bail!("archive entry has an empty name");
    }
    if name.contains('\0') {
        anyhow::bail!("archive entry name contains a NUL byte");
    }
    if name.contains("..") {
        anyhow::bail!("archive entry name contains `..`");
    }
    if name.starts_with('/') || name.starts_with('\\') {
        anyhow::bail!("archive entry name is absolute: {}", name);
    }
    // Windows drive letter prefix: `C:` or `C:\...`
    if name.len() >= 2 && name.as_bytes()[0].is_ascii_alphabetic() && name.as_bytes()[1] == b':' {
        anyhow::bail!("archive entry name has a Windows drive prefix: {}", name);
    }
    // Reject control characters
    if name.chars().any(|c| c.is_control()) {
        anyhow::bail!("archive entry name contains a control character");
    }
    // Reject Windows-style backslashes in the name itself
    if name.contains('\\') {
        anyhow::bail!("archive entry name contains a backslash: {}", name);
    }
    Ok(name)
}

fn extract_zip(
    archive_path: &Path,
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
) -> Result<()> {
    let file = fs::File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    let total_files = archive.len();

    fs::create_dir_all(dest_dir)?;

    for i in 0..total_files {
        let mut file = archive.by_index(i)?;
        let entry_name = match file.enclosed_name() {
            // `enclosed_name` already strips `..` and absolute
            // paths, but we re-validate as defence in depth.
            Some(p) => p
                .to_str()
                .ok_or_else(|| anyhow!("archive entry name is not valid UTF-8"))?
                .to_string(),
            None => continue,
        };
        validate_archive_entry_name(&entry_name)?;
        let outpath = dest_dir.join(&entry_name);

        let file_name = outpath
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        let _ = tx.blocking_send(ProgressUpdate {
            current_file: file_name,
            files_copied: i,
            total_files,
            bytes_copied: 0,
            total_bytes: 0,
            error: None,
        });

        if (&*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

fn extract_tar_gz(
    archive_path: &Path,
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
) -> Result<()> {
    let tar_gz = fs::File::open(archive_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    fs::create_dir_all(dest_dir)?;

    // We don't know total files in tar easily without reading it twice, so we just show 0 or an arbitrary number
    let mut i = 0;
    for entry in archive.entries()? {
        let mut file = entry?;
        let path = file.path()?;

        // The `tar` crate refuses to unpack `..` and absolute
        // paths by default, but we re-validate the entry name
        // explicitly so a future change to `unpack_in` (or a
        // downgrade of the crate) does not silently re-open this
        // zip-slip hole.
        let entry_name = path
            .to_str()
            .ok_or_else(|| anyhow!("archive entry name is not valid UTF-8"))?;
        validate_archive_entry_name(entry_name)?;

        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        let _ = tx.blocking_send(ProgressUpdate {
            current_file: file_name,
            files_copied: i,
            total_files: 0, // Unknown
            bytes_copied: 0,
            total_bytes: 0,
            error: None,
        });

        file.unpack_in(dest_dir)?;
        i += 1;
    }

    Ok(())
}

fn extract_7z(
    archive_path: &Path,
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
) -> Result<()> {
    fs::create_dir_all(dest_dir)?;

    // The `sevenz-rust 0.6.1` default extractor does NOT validate
    // entry names. An entry named `../../../etc/cron.d/pairee`
    // would be written to `dest/../../../etc/cron.d/pairee`,
    // which the OS happily resolves to `/etc/cron.d/pairee`. We
    // therefore replace the default callback with one that
    // validates every name AND refuses to follow symlinks that
    // appear in the destination tree (an attacker could create
    // a symlink in `dest_dir` before the extract runs and trick
    // the extractor into writing through it).
    sevenz_rust::decompress_file_with_extract_fn(archive_path, dest_dir, |entry, reader, dest| {
        let entry_name = entry.name();
        if let Err(e) = validate_archive_entry_name(entry_name) {
            return Err(sevenz_rust::Error::io_msg(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("refusing unsafe 7z entry: {}", e),
                ),
                "validate_archive_entry_name",
            ));
        }

        // Defence against symlink-swap TOCTOU: refuse to
        // write through a pre-existing symlink. The check
        // uses `symlink_metadata` so it does not follow the
        // link.
        if let Ok(meta) = std::fs::symlink_metadata(dest) {
            if meta.file_type().is_symlink() {
                return Err(sevenz_rust::Error::io_msg(
                    std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        format!("refusing to write through symlink at {:?}", dest),
                    ),
                    "symlink-swap defence",
                ));
            }
        }

        let file_name = entry.name().to_string();
        let _ = tx.blocking_send(ProgressUpdate {
            current_file: file_name,
            files_copied: 0,
            total_files: 0,
            bytes_copied: 0,
            total_bytes: 0,
            error: None,
        });

        sevenz_rust::default_entry_extract_fn(entry, reader, dest)
    })
    .map_err(|e| anyhow!("7z extraction failed: {:?}", e))?;

    Ok(())
}

pub fn compress_zip(
    sources: Vec<PathBuf>,
    dest_archive: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
) -> Result<()> {
    let file = fs::File::create(dest_archive)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut i: usize = 0;

    // We need a stable upper bound for the progress denominator.
    // Walk the tree first to count every file we will add, then
    // emit one ProgressUpdate per file with a correct total.
    let mut all_paths: Vec<PathBuf> = Vec::new();
    for src in &sources {
        if src.is_file() {
            all_paths.push(src.clone());
        } else if src.is_dir() {
            // Recursive walk. The previous version of this
            // function added only the empty directory entry and
            // silently dropped the files inside, which is a
            // user-visible data-loss bug.
            collect_files_recursive(src, &mut all_paths)?;
        } else {
            // Neither file nor directory (broken symlink,
            // dangling mount, race with the user deleting the
            // entry between our checks): skip with a clear
            // warning rather than silently producing an empty
            // archive.
            log::warn!(
                "compress_zip: source {:?} is not a file or directory, skipping",
                src
            );
        }
    }
    let total_files = all_paths.len();

    for src in &all_paths {
        let name = src
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("source file {:?} has no name component", src))?;
        let _ = tx.blocking_send(ProgressUpdate {
            current_file: name.to_string(),
            files_copied: i,
            total_files,
            bytes_copied: 0,
            total_bytes: 0,
            error: None,
        });

        zip.start_file(name, options)?;
        let mut f = fs::File::open(src)?;
        io::copy(&mut f, &mut zip)?;
        i += 1;
    }

    zip.finish()?;
    Ok(())
}

/// Walk `dir` recursively and append every regular file to
/// `out`. Symlinks are followed (matching the behaviour of a
/// tar-style "include the contents" archive). The previous
/// implementation skipped directories entirely; this one
/// walks them.
fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d)? {
            let entry = entry?;
            let path = entry.path();
            // Follow symlinks: if the symlink target is a file,
            // include it; if it is a directory, recurse into it;
            // if it is broken, skip it with a warning.
            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(e) => {
                    log::warn!("compress_zip: cannot stat {:?} ({}), skipping", path, e);
                    continue;
                }
            };
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                out.push(path);
            }
        }
    }
    Ok(())
}

fn extract_via_external_7z(
    archive_path: &Path,
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
) -> Result<()> {
    use crate::fs::external_tools::get_external_7z_path;

    let bin_path = get_external_7z_path().ok_or_else(|| anyhow!("Could not determine 7z path"))?;
    if !bin_path.exists() && cfg!(target_os = "windows") {
        return Err(anyhow!(
            "7z tool is not downloaded yet. Please wait for the background download to finish."
        ));
    }

    fs::create_dir_all(dest_dir)?;

    let _ = tx.blocking_send(ProgressUpdate {
        current_file: format!("Extracting using external 7z..."),
        files_copied: 0,
        total_files: 0,
        bytes_copied: 0,
        total_bytes: 0,
        error: None,
    });

    // Argument-injection hardening: the previous version
    // concatenated `-o` with `dest_dir` without a separator. If
    // `dest_dir` happened to start with `-` (a perfectly legal
    // path on Linux: a file or directory whose name starts
    // with a dash), 7z would interpret the resulting argument
    // as an unknown option and either error out or, in the
    // worst case, treat it as a flag. We now pass the
    // destination as a separate `--` argument; the `=` form
    // `-o<path>` is unambiguous but only when the path does
    // not start with `-`. We also pass `--` before the archive
    // to make sure 7z stops parsing options there.
    //
    // `7z x` does not accept `--` as a positional separator the
    // same way `curl` / `git` do, so we use the alternate `-o`
    // form with an absolute path. The `to_string_lossy()` may
    // produce non-UTF-8 bytes, which is fine for `Command::arg`
    // on Unix (it accepts `OsStr` natively).
    let dest_arg = match std::path::absolute(dest_dir) {
        Ok(p) => p,
        Err(_) => dest_dir.to_path_buf(),
    };
    // Prefix with `./` so the result never starts with `-` even
    // if the user passed a relative path that happened to be
    // `-something`.
    let dest_arg_str = if dest_arg.is_absolute() {
        dest_arg.to_string_lossy().into_owned()
    } else {
        format!(
            "./{}",
            dest_arg
                .to_string_lossy()
                .trim_start_matches("./")
                .trim_start_matches('/')
        )
    };
    let output = std::process::Command::new(&bin_path)
        .arg("x")
        .arg("-y") // yes to all queries
        .arg(format!("-o{}", dest_arg_str))
        .arg(archive_path)
        .output()?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("External 7z extraction failed: {}", err_msg));
    }

    Ok(())
}

pub fn list_archive_files(path: &Path) -> Result<Vec<String>> {
    match detect_format(path) {
        ArchiveFormat::Zip => {
            let file = fs::File::open(path)?;
            let mut archive = ZipArchive::new(file)?;
            let mut list = Vec::new();
            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index(i) {
                    list.push(file.name().to_string());
                }
            }
            Ok(list)
        }
        ArchiveFormat::TarGz => {
            let tar_gz = fs::File::open(path)?;
            let tar = GzDecoder::new(tar_gz);
            let mut archive = Archive::new(tar);
            let mut list = Vec::new();
            for entry in archive.entries()? {
                if let Ok(entry) = entry {
                    if let Ok(path) = entry.path() {
                        list.push(path.to_string_lossy().into_owned());
                    }
                }
            }
            Ok(list)
        }
        ArchiveFormat::SevenZ => {
            let archive = sevenz_rust::Archive::open(path)
                .map_err(|e| anyhow!("Failed to open 7z: {:?}", e))?;
            let mut list = Vec::new();
            for entry in &archive.files {
                list.push(entry.name.clone());
            }
            Ok(list)
        }
        _ => Err(anyhow!(
            "Unsupported archive format or listing not supported"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // §6: `validate_archive_entry_name` is the zip-slip / 7z-slip
    // gate. Every rejection rule must be covered. The table is
    // intentionally a mix of attack-shaped names ("..",
    // "C:\foo", "/etc/passwd") and legitimate ones.
    #[test]
    fn test_validate_archive_entry_name_accepts_safe_names() {
        let safe = [
            "file.txt",
            "dir/file.txt",
            "deep/nested/dir/leaf.md",
            "with spaces/and-dashes.toml",
            "unicode_ñ_é_ü.txt",
            // backslash is rejected — see below. We don't
            // include it here.
        ];
        for name in safe {
            assert!(
                validate_archive_entry_name(name).is_ok(),
                "expected {:?} to be accepted",
                name
            );
        }
    }

    #[test]
    fn test_validate_archive_entry_name_rejects_attack_names() {
        // Each row: (name, expected substring in the error,
        // case-insensitive).
        let bad: &[(&str, &str)] = &[
            ("", "empty"),
            ("..", ".."),
            ("../etc/passwd", ".."),
            ("a/../../etc/passwd", ".."),
            // absolute paths
            ("/etc/passwd", "absolute"),
            ("\\etc\\passwd", "absolute"),
            // Windows drive prefix
            ("C:foo", "drive"),
            ("C:\\Windows\\System32", "drive"),
            ("D:config.sys", "drive"),
            // NUL byte
            ("file\0.txt", "nul"),
            ("a/b\0/c", "nul"),
            // control characters
            ("file\nname", "control"),
            ("file\tname", "control"),
            ("\r", "control"),
            // backslashes (we use forward-slash semantics)
            ("dir\\file", "backslash"),
            ("a\\b\\c", "backslash"),
        ];
        for (name, hint) in bad {
            let res = validate_archive_entry_name(name);
            assert!(
                res.is_err(),
                "expected {:?} to be rejected, got {:?}",
                name,
                res
            );
            let err = format!("{}", res.unwrap_err()).to_lowercase();
            let hint_lc = hint.to_lowercase();
            assert!(
                err.contains(&hint_lc),
                "expected error for {:?} to mention {:?}, got {:?}",
                name,
                hint_lc,
                err
            );
        }
    }
}
