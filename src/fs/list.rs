use super::entry::FileEntry;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

#[cfg(target_os = "windows")]
fn read_directory_as_admin(path: &Path) -> Result<Vec<FileEntry>> {
    use std::process::Command;

    // We avoid embedding the user-supplied path inside a PowerShell
    // string (which would require a fragile, multi-level escape
    // against `"`, `` ` ``, `$`, `'`, etc.) by writing the script
    // to a `.ps1` file and passing the path as a separate
    // `-ArgumentList` element. The only string interpolation that
    // remains is the path *as a single quoted argument* to
    // `Start-Process`, where we only need to escape the single
    // quote (PowerShell literal-string escape: two single quotes).
    let temp_dir = std::env::temp_dir();
    let script_file = temp_dir.join(format!("pairee_dir_{}.ps1", std::process::id()));
    let temp_file = temp_dir.join(format!("pairee_dir_{}.txt", std::process::id()));

    // PowerShell script. It receives the source path via $args[0]
    // and the output path via $args[1]; no string interpolation of
    // user data is performed.
    const PS_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$srcPath = $args[0]
$outPath = $args[1]
Get-ChildItem -Path $srcPath -Force | ForEach-Object {
    "$($_.Name)|$($_.Length)|$($_.Mode)|$($_.LastWriteTime.Ticks)"
} | Out-File -FilePath $outPath -Encoding utf8
"#;
    std::fs::write(&script_file, PS_SCRIPT).with_context(|| {
        format!(
            "failed to write elevated-helper script to {:?}",
            script_file
        )
    })?;

    // Escape single quotes for PowerShell literal strings (the only
    // metacharacter that can break out of a single-quoted argument
    // token). Backticks, dollar signs, and double quotes are all
    // inert inside '...'.
    let script_escaped = script_file.to_string_lossy().replace('\'', "''");
    let path_escaped = path.to_string_lossy().replace('\'', "''");
    let out_escaped = temp_file.to_string_lossy().replace('\'', "''");

    let ps_run = format!(
        "Start-Process powershell -ArgumentList '-NoProfile', '-File', '{}', '{}', '{}' -Verb RunAs -WindowStyle Hidden -Wait",
        script_escaped, path_escaped, out_escaped
    );
    let status = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &ps_run])
        .status()?;

    let _ = std::fs::remove_file(&script_file);
    if status.success() && temp_file.exists() {
        let content = std::fs::read_to_string(&temp_file)?;
        let _ = std::fs::remove_file(&temp_file);
        let mut entries = Vec::new();
        for line in content.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                let size: u64 = parts[1].parse().unwrap_or(0);
                let mode = parts[2];
                let is_dir = mode.contains('d') || mode.contains('D');
                let ticks: i64 = parts[3].parse().unwrap_or(0);

                let modified = if ticks > 0 {
                    let epoch_ticks = 621355968000000000i64;
                    let unix_ticks = ticks - epoch_ticks;
                    let secs = unix_ticks / 10_000_000;
                    if secs > 0 {
                        Some(
                            std::time::SystemTime::UNIX_EPOCH
                                + std::time::Duration::from_secs(secs as u64),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                };

                let entry_path = path.join(&name);
                entries.push(FileEntry {
                    name,
                    path: entry_path,
                    size,
                    is_dir,
                    is_symlink: mode.contains('l') || mode.contains('L'),
                    modified,
                });
            }
        }
        Ok(entries)
    } else {
        anyhow::bail!("Failed to read directory as admin")
    }
}

#[cfg(not(target_os = "windows"))]
fn read_directory_as_admin(path: &Path) -> Result<Vec<FileEntry>> {
    use std::process::Command;

    // We used to shell out to `sudo python3 -c '<script>' <path>`, which
    // (a) required python3 to be installed on the user's machine (it is
    // not by default on minimal Fedora Server, RHEL minimal, Alpine, etc.),
    // (b) required a sudoers entry that allows the user to run python3
    // without a TTY, and (c) was a multi-language, multi-process pile that
    // was hard to debug when it broke.
    //
    // The cleaner approach: re-exec *ourselves* under `sudo` with a
    // dedicated `--list-dir-elevated <path>` flag, implemented by
    // `list_dir_elevated` below. The path is passed as a separate
    // `Command::arg` (the safe boundary), there is no shell involvement,
    // and the only external dependency is `sudo` itself — which we
    // already require for the other elevated operations.
    let exe = std::env::current_exe().context("cannot determine current exe path")?;

    let output = Command::new("sudo")
        .arg(&exe)
        .arg("--list-dir-elevated")
        .arg(path)
        .output()
        .map_err(|e| {
            anyhow::anyhow!(
                "failed to invoke `sudo` for elevated directory listing: {}. \
                 Make sure `sudo` is installed and the user is in sudoers.",
                e
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "elevated directory listing failed (exit {}): {}",
            output.status,
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let entries: Vec<FileEntry> = serde_json::from_str(stdout.trim())
        .context("failed to parse elevated directory listing output")?;
    Ok(entries)
}

/// Listing routine executed by the re-exec'd, elevated child process
/// (triggered by `main` when `--list-dir-elevated <path>` is on argv).
///
/// Returns a `Vec<FileEntry>` ready to be serialized as JSON and parsed
/// by the unprivileged parent.
pub fn list_dir_elevated(path: &Path) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    for entry in
        fs::read_dir(path).with_context(|| format!("elevated read_dir failed for {:?}", path))?
    {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                // A single unreadable entry should not abort the whole
                // listing — this is "show me what you can", not "fail
                // closed on first error".
                log::warn!("skipping unreadable entry under {:?}: {}", path, e);
                continue;
            }
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "." || name == ".." {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                log::warn!("skipping {:?}: metadata error {}", entry.path(), e);
                continue;
            }
        };
        let modified = metadata.modified().ok();
        let entry_path = entry.path();
        entries.push(FileEntry {
            name,
            path: entry_path,
            size: metadata.len(),
            is_dir: metadata.is_dir(),
            is_symlink: metadata.file_type().is_symlink(),
            modified,
        });
    }
    Ok(entries)
}

fn cmp_natural(a: &str, b: &str, case_sensitive: bool) -> std::cmp::Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(&ca), Some(&cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let mut num_a: u64 = 0;
                    while let Some(&c) = a_chars.peek() {
                        if c.is_ascii_digit() {
                            num_a = num_a
                                .saturating_mul(10)
                                .saturating_add(c.to_digit(10).unwrap() as u64);
                            a_chars.next();
                        } else {
                            break;
                        }
                    }
                    let mut num_b: u64 = 0;
                    while let Some(&c) = b_chars.peek() {
                        if c.is_ascii_digit() {
                            num_b = num_b
                                .saturating_mul(10)
                                .saturating_add(c.to_digit(10).unwrap() as u64);
                            b_chars.next();
                        } else {
                            break;
                        }
                    }
                    match num_a.cmp(&num_b) {
                        std::cmp::Ordering::Equal => continue,
                        ord => return ord,
                    }
                } else {
                    let mut char_a = a_chars.next().unwrap();
                    let mut char_b = b_chars.next().unwrap();
                    if !case_sensitive {
                        char_a = char_a.to_lowercase().next().unwrap_or(char_a);
                        char_b = char_b.to_lowercase().next().unwrap_or(char_b);
                    }
                    match char_a.cmp(&char_b) {
                        std::cmp::Ordering::Equal => continue,
                        ord => return ord,
                    }
                }
            }
        }
    }
}

fn cmp_standard(a: &str, b: &str, case_sensitive: bool) -> std::cmp::Ordering {
    if case_sensitive {
        a.cmp(b)
    } else {
        a.to_lowercase().cmp(&b.to_lowercase())
    }
}

fn entry_sort_key_ext(entry: &FileEntry, sort_folder_names_by_extension: bool) -> String {
    if entry.is_dir && !sort_folder_names_by_extension {
        String::new()
    } else {
        std::path::Path::new(&entry.name)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    }
}

pub fn sort_entries(
    entries: &mut Vec<FileEntry>,
    sort_field: crate::app::state::SortField,
    sort_reverse: bool,
    case_sensitive_sort: bool,
    treat_digits_as_numbers: bool,
    sort_folder_names_by_extension: bool,
) {
    use crate::app::state::SortField;

    if matches!(sort_field, SortField::Unsorted) {
        // No sorting — just pin ".." first
        if let Some(pos) = entries.iter().position(|e| e.name == "..") {
            let dotdot = entries.remove(pos);
            entries.insert(0, dotdot);
        }
    } else {
        entries.sort_by(|a, b| {
            // ".." is always first
            if a.name == ".." {
                return std::cmp::Ordering::Less;
            }
            if b.name == ".." {
                return std::cmp::Ordering::Greater;
            }

            // Extension sort: folders and files are mixed by extension key
            // All other sorts: directories sort before files
            let dir_order = if matches!(sort_field, SortField::Extension) {
                std::cmp::Ordering::Equal
            } else {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Equal,
                }
            };

            if dir_order != std::cmp::Ordering::Equal {
                return if sort_reverse {
                    dir_order.reverse()
                } else {
                    dir_order
                };
            }

            let name_ord = match sort_field {
                SortField::Name => {
                    if treat_digits_as_numbers {
                        cmp_natural(&a.name, &b.name, case_sensitive_sort)
                    } else {
                        cmp_standard(&a.name, &b.name, case_sensitive_sort)
                    }
                }
                SortField::Extension => {
                    let ext_a = entry_sort_key_ext(a, sort_folder_names_by_extension);
                    let ext_b = entry_sort_key_ext(b, sort_folder_names_by_extension);
                    let ext_ord = if treat_digits_as_numbers {
                        cmp_natural(&ext_a, &ext_b, case_sensitive_sort)
                    } else {
                        cmp_standard(&ext_a, &ext_b, case_sensitive_sort)
                    };
                    if ext_ord == std::cmp::Ordering::Equal {
                        // Secondary sort by name when extensions are equal
                        if treat_digits_as_numbers {
                            cmp_natural(&a.name, &b.name, case_sensitive_sort)
                        } else {
                            cmp_standard(&a.name, &b.name, case_sensitive_sort)
                        }
                    } else {
                        ext_ord
                    }
                }
                SortField::Size => {
                    // Dirs: compare name; files: compare size
                    if a.is_dir && b.is_dir {
                        cmp_standard(&a.name, &b.name, case_sensitive_sort)
                    } else {
                        a.size.cmp(&b.size)
                    }
                }
                SortField::Date => {
                    let t_a = a.modified.map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                    });
                    let t_b = b.modified.map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                    });
                    t_a.cmp(&t_b)
                }
                SortField::Unsorted => std::cmp::Ordering::Equal,
            };

            if sort_reverse {
                name_ord.reverse()
            } else {
                name_ord
            }
        });
    }
}

/// Reads directory contents with default sort (Name, ascending, no folder-by-ext, with dotdot).
/// Deprecated: use `read_directory_ext` for full panel settings support.
#[allow(dead_code)]
pub fn read_directory(
    path: &Path,
    show_hidden: bool,
    case_sensitive_sort: bool,
    treat_digits_as_numbers: bool,
    _sorting_collation: &str,
    req_admin_reading: bool,
) -> Result<Vec<FileEntry>> {
    read_directory_ext(
        path,
        show_hidden,
        case_sensitive_sort,
        treat_digits_as_numbers,
        _sorting_collation,
        req_admin_reading,
        crate::app::state::SortField::Name,
        false,
        false,
        true,
    )
}

/// Extended directory reader with full panel settings support.
pub fn read_directory_ext(
    path: &Path,
    show_hidden: bool,
    case_sensitive_sort: bool,
    treat_digits_as_numbers: bool,
    _sorting_collation: &str,
    req_admin_reading: bool,
    sort_field: crate::app::state::SortField,
    sort_reverse: bool,
    sort_folder_names_by_extension: bool,
    show_dotdot_in_root_folders: bool,
) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    // 1. Add ".." parent directory entry
    //    Always added if a parent exists; or if show_dotdot_in_root_folders is enabled.
    let has_parent = path.parent().is_some();
    if has_parent {
        let parent = path.parent().unwrap();
        entries.push(FileEntry {
            name: "..".to_string(),
            path: parent.to_path_buf(),
            size: 0,
            is_dir: true,
            is_symlink: false,
            modified: None,
        });
    } else if show_dotdot_in_root_folders {
        // Insert a ".." that stays in the current root (navigating up from root stays at root).
        entries.push(FileEntry {
            name: "..".to_string(),
            path: path.to_path_buf(),
            size: 0,
            is_dir: true,
            is_symlink: false,
            modified: None,
        });
    }

    // 2. Read directory contents
    let read_res = fs::read_dir(path);
    let read_entries = match read_res {
        Ok(read_dir) => {
            let mut items = Vec::new();
            for entry in read_dir.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();

                // Skip hidden files if show_hidden is not enabled
                if !show_hidden && name.starts_with('.') {
                    continue;
                }

                let metadata = entry.metadata().ok();
                let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let is_symlink = metadata.as_ref().map(|m| m.is_symlink()).unwrap_or(false);
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = metadata.and_then(|m| m.modified().ok());

                items.push(FileEntry {
                    name,
                    path: entry.path(),
                    size,
                    is_dir,
                    is_symlink,
                    modified,
                });
            }
            Ok(items)
        }
        Err(e) => {
            if req_admin_reading {
                read_directory_as_admin(path)
            } else {
                Err(anyhow::anyhow!(e))
            }
        }
    };

    let mut read_entries = read_entries.context(format!("Failed to read directory: {:?}", path))?;
    entries.append(&mut read_entries);

    // 3. Sort entries using the extracted public function
    sort_entries(
        &mut entries,
        sort_field,
        sort_reverse,
        case_sensitive_sort,
        treat_digits_as_numbers,
        sort_folder_names_by_extension,
    );

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmp_natural() {
        // Natural sorting: numbers are compared numerically
        assert_eq!(
            cmp_natural("file2", "file10", false),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            cmp_natural("file10", "file2", false),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            cmp_natural("file2", "file2", false),
            std::cmp::Ordering::Equal
        );

        // Case-insensitive by default in cmp_natural when case_sensitive=false
        assert_eq!(
            cmp_natural("File2", "file2", false),
            std::cmp::Ordering::Equal
        );

        // Case-sensitive in cmp_natural when case_sensitive=true
        assert_eq!(
            cmp_natural("File2", "file2", true),
            std::cmp::Ordering::Less
        ); // 'F' < 'f'
    }

    #[test]
    fn test_cmp_standard() {
        // Standard sorting: alphabetical comparison
        assert_eq!(
            cmp_standard("file10", "file2", false),
            std::cmp::Ordering::Less
        ); // '1' < '2'

        // Case-insensitive standard sorting
        assert_eq!(
            cmp_standard("File", "file", false),
            std::cmp::Ordering::Equal
        );

        // Case-sensitive standard sorting
        assert_eq!(cmp_standard("File", "file", true), std::cmp::Ordering::Less); // 'F' < 'f'
    }
}
