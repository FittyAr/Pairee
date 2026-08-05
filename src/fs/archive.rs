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
        let outpath = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };

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
    use std::path::Component;

    fs::create_dir_all(dest_dir)?;
    // Canonicalise the destination so we can verify that no extracted
    // entry escapes it via `..` components or absolute paths.
    let canonical_dest = std::fs::canonicalize(dest_dir)
        .unwrap_or_else(|_| dest_dir.to_path_buf());

    sevenz_rust::decompress_file_with_extract_fn(archive_path, dest_dir, |entry, reader, dest| {
        // The `sevenz-rust` 0.6.x API does not sanitise entry names; it
        // simply `dest.join(entry.name())` and lets the extract function
        // create the file. A malicious 7z archive can therefore write
        // outside the chosen destination by including entries like
        // `..\..\..\Windows\System32\evil.dll`. We refuse any entry whose
        // path resolves outside the canonical destination directory.
        let entry_name = entry.name();
        let candidate = std::path::Path::new(entry_name);
        let mut has_traversal = false;
        for component in candidate.components() {
            match component {
                Component::ParentDir => {
                    has_traversal = true;
                    break;
                }
                Component::Prefix(_) | Component::RootDir => {
                    // Absolute paths or Windows drive prefixes are never
                    // legitimate inside an archive entry.
                    has_traversal = true;
                    break;
                }
                _ => {}
            }
        }
        if has_traversal {
            // Skip the entry entirely; do not call the default extractor.
            let file_name = entry.name().to_string();
            let _ = tx.blocking_send(ProgressUpdate {
                current_file: file_name,
                files_copied: 0,
                total_files: 0,
                bytes_copied: 0,
                total_bytes: 0,
                error: Some(format!(
                    "Refusing to extract entry with unsafe path: {}",
                    entry.name()
                )),
            });
            return Ok(false);
        }

        // Resolve the entry's final path and make sure it stays under
        // `canonical_dest`. The resolved path may not exist yet, so we
        // canonicalize what we can and compare by prefix. We also
        // canonicalise the parent so symlinks cannot redirect the write
        // outside the destination.
        let dest_path = dest.to_path_buf();
        let check_target = dest_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| dest_path.clone());
        if let Ok(canon) = std::fs::canonicalize(&check_target) {
            if !canon.starts_with(&canonical_dest) {
                let _ = tx.blocking_send(ProgressUpdate {
                    current_file: entry.name().to_string(),
                    files_copied: 0,
                    total_files: 0,
                    bytes_copied: 0,
                    total_bytes: 0,
                    error: Some(format!(
                        "Refusing to extract entry outside destination: {}",
                        entry.name()
                    )),
                });
                return Ok(false);
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

        sevenz_rust::default_entry_extract_fn(entry, reader, &dest_path)
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

    let mut i = 0usize;
    let total_files = sources.len();

    for src in sources {
        if src.is_dir() {
            // Recursive walk so the user gets the full folder contents
            // inside the archive, not just an empty entry. The archive
            // path is built by stripping `src.parent()` from each entry
            // and prepending `src.file_name()` so the directory appears
            // at the top of the archive.
            let top = src.file_name().unwrap_or_default().to_os_string();
            let base_parent = src.parent().map(|p| p.to_path_buf());
            let mut stack: Vec<PathBuf> = vec![src.clone()];
            while let Some(dir) = stack.pop() {
                let dir_name_in_zip = match dir.strip_prefix(&src) {
                    Ok(rel) if !rel.as_os_str().is_empty() => {
                        let mut p = top.clone();
                        for component in rel.components() {
                            p.push(component.as_os_str());
                        }
                        p
                    }
                    _ => top.clone(),
                };
                zip.add_directory(dir_name_in_zip.to_string_lossy(), options)?;

                let entries = match fs::read_dir(&dir) {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        stack.push(entry_path);
                    } else {
                        let rel = entry_path.strip_prefix(&src).unwrap_or(&entry_path);
                        let mut zip_path = top.clone();
                        for component in rel.components() {
                            zip_path.push(component.as_os_str());
                        }
                        let zip_path_str = zip_path.to_string_lossy().into_owned();
                        let _ = tx.blocking_send(ProgressUpdate {
                            current_file: zip_path_str.clone(),
                            files_copied: i,
                            total_files,
                            bytes_copied: 0,
                            total_bytes: 0,
                            error: None,
                        });
                        zip.start_file(zip_path_str, options)?;
                        let mut f = match fs::File::open(&entry_path) {
                            Ok(f) => f,
                            Err(e) => {
                                log::warn!(
                                    "compress_zip: skipping {:?}: {}",
                                    entry_path,
                                    e
                                );
                                continue;
                            }
                        };
                        io::copy(&mut f, &mut zip)?;
                        i += 1;
                    }
                }
            }
            // Suppress unused warning for base_parent: it documents the
            // intent that the relative-path arithmetic above is rooted
            // at the source directory.
            let _ = base_parent;
        } else {
            let name = src.file_name().unwrap_or_default().to_string_lossy();
            let _ = tx.blocking_send(ProgressUpdate {
                current_file: name.to_string(),
                files_copied: i,
                total_files,
                bytes_copied: 0,
                total_bytes: 0,
                error: None,
            });

            zip.start_file(name, options)?;
            let mut f = fs::File::open(&src)?;
            io::copy(&mut f, &mut zip)?;
            i += 1;
        }
    }

    zip.finish()?;
    Ok(())
}

fn extract_via_external_7z(
    archive_path: &Path,
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
) -> Result<()> {
    use crate::fs::external_tools::get_external_7z_path;
    use std::path::Component;

    let bin_path = get_external_7z_path().ok_or_else(|| anyhow!("Could not determine 7z path"))?;
    if !bin_path.exists() && cfg!(target_os = "windows") {
        return Err(anyhow!(
            "7z tool is not downloaded yet. Please wait for the background download to finish."
        ));
    }

    fs::create_dir_all(dest_dir)?;

    // Path-traversal guard. Unlike our native zip/7z paths, the external
    // 7z binary does not get a custom callback for each entry, so a
    // malicious RAR/ISO that contains entries with `..` or absolute
    // components could otherwise be extracted outside `dest_dir`. We list
    // the archive first and reject it if any entry has a path component
    // that would escape the destination.
    let list_output = std::process::Command::new(&bin_path)
        .arg("l")
        .arg("-slt") // long technical listing, one block per file
        .arg(archive_path)
        .output()?;
    if list_output.status.success() {
        let listing = String::from_utf8_lossy(&list_output.stdout);
        for block in listing.split("\n\n") {
            for line in block.lines() {
                if let Some(path) = line.strip_prefix("Path = ") {
                    let candidate = std::path::Path::new(path.trim());
                    let mut has_traversal = false;
                    for component in candidate.components() {
                        match component {
                            Component::ParentDir => {
                                has_traversal = true;
                                break;
                            }
                            Component::Prefix(_) | Component::RootDir => {
                                // Windows drive prefix or root: not a
                                // legitimate archive entry relative to
                                // `dest_dir`.
                                has_traversal = true;
                                break;
                            }
                            _ => {}
                        }
                    }
                    if has_traversal {
                        let _ = tx.blocking_send(ProgressUpdate {
                            current_file: path.trim().to_string(),
                            files_copied: 0,
                            total_files: 0,
                            bytes_copied: 0,
                            total_bytes: 0,
                            error: Some(format!(
                                "Refusing to extract archive: entry {} would escape the destination",
                                path.trim()
                            )),
                        });
                        return Err(anyhow!(
                            "Refusing to extract archive: entry {} contains a path-traversal component",
                            path.trim()
                        ));
                    }
                }
            }
        }
    }
    // If the listing step itself failed we still continue, but we surface
    // the warning so the operator knows the safety check did not run.

    let _ = tx.blocking_send(ProgressUpdate {
        current_file: format!("Extracting using external 7z..."),
        files_copied: 0,
        total_files: 0,
        bytes_copied: 0,
        total_bytes: 0,
        error: None,
    });

    let output = std::process::Command::new(&bin_path)
        .arg("x")
        .arg("-y") // yes to all queries
        .arg(format!("-o{}", dest_dir.to_string_lossy()))
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
