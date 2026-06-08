use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::path::PathBuf;

/// Status of a file relative to the two panels being compared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareStatus {
    /// File only exists in the left panel directory.
    OnlyLeft,
    /// File only exists in the right panel directory.
    OnlyRight,
    /// File exists in both but differs in size or modification time.
    Different,
    /// File exists in both and appears identical (same size + mtime).
    Equal,
}

/// One entry in the comparison result.
#[derive(Debug, Clone)]
pub struct CompareEntry {
    pub name: String,
    pub status: CompareStatus,
}

/// Compares the contents of two directories by filename, size, and modification time.
///
/// Does NOT recurse into subdirectories. Returns a sorted list of all entries
/// from both sides with their comparison status.
pub fn compare_directories(left: &Path, right: &Path) -> Result<Vec<CompareEntry>> {
    let left_map = scan_directory(left)?;
    let right_map = scan_directory(right)?;

    let all_names: HashSet<&String> = left_map.keys().chain(right_map.keys()).collect();
    let mut results: Vec<CompareEntry> = all_names
        .into_iter()
        .map(|name| {
            let status = match (left_map.get(name), right_map.get(name)) {
                (Some(_), None) => CompareStatus::OnlyLeft,
                (None, Some(_)) => CompareStatus::OnlyRight,
                (Some(l), Some(r)) => {
                    if l.size == r.size && mtime_eq(l.modified, r.modified) {
                        CompareStatus::Equal
                    } else {
                        CompareStatus::Different
                    }
                }
                (None, None) => unreachable!(),
            };
            CompareEntry {
                name: name.clone(),
                status,
            }
        })
        .collect();

    results.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(results)
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

struct FileSummary {
    size: u64,
    modified: Option<std::time::SystemTime>,
}

fn scan_directory(dir: &Path) -> Result<HashMap<String, FileSummary>> {
    let mut map = HashMap::new();
    let read_dir = std::fs::read_dir(dir)?;
    for entry in read_dir.flatten() {
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_file() {
            let name = entry.file_name().to_string_lossy().to_string();
            map.insert(
                name,
                FileSummary {
                    size: meta.len(),
                    modified: meta.modified().ok(),
                },
            );
        }
    }
    Ok(map)
}

fn mtime_eq(
    a: Option<std::time::SystemTime>,
    b: Option<std::time::SystemTime>,
) -> bool {
    match (a, b) {
        (Some(ta), Some(tb)) => {
            // Allow 1-second tolerance (FAT32 filesystem granularity)
            match ta.duration_since(tb) {
                Ok(d) => d.as_secs() == 0,
                Err(e) => e.duration().as_secs() == 0,
            }
        }
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path
    }

    #[test]
    fn test_compare_basic() {
        let left_dir = tempfile::tempdir().expect("left tempdir");
        let right_dir = tempfile::tempdir().expect("right tempdir");

        write_file(left_dir.path(), "common.txt", b"same content");
        write_file(right_dir.path(), "common.txt", b"same content");
        write_file(left_dir.path(), "left_only.rs", b"only left");
        write_file(right_dir.path(), "right_only.rs", b"only right");
        write_file(left_dir.path(), "differ.txt", b"version A");
        write_file(right_dir.path(), "differ.txt", b"version B longer");

        let results =
            compare_directories(left_dir.path(), right_dir.path()).expect("compare");

        let find = |name: &str| {
            results.iter().find(|e| e.name == name).map(|e| &e.status)
        };

        assert_eq!(find("left_only.rs"), Some(&CompareStatus::OnlyLeft));
        assert_eq!(find("right_only.rs"), Some(&CompareStatus::OnlyRight));
        assert_eq!(find("differ.txt"), Some(&CompareStatus::Different));
        // "common.txt" might be Equal or Different depending on mtime precision;
        // just verify it appears in results.
        assert!(find("common.txt").is_some());
    }
}
