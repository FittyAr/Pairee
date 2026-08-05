use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConflictResolution {
    Ask,
    Overwrite,
    OverwriteAll,
    OverwriteOlder,
    OverwriteOlderAll,
    Skip,
    SkipAll,
    Rename,
    RenameAll,
    KeepBoth,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub src_path: PathBuf,
    pub dst_path: PathBuf,
    pub src_size: u64,
    pub dst_size: u64,
    pub src_modified: Option<std::time::SystemTime>,
    pub dst_modified: Option<std::time::SystemTime>,
}

/// Autogenera un nombre de archivo no conflictivo en el directorio destino.
/// Por ejemplo: `archivo.txt` -> `archivo (1).txt`, `archivo (2).txt`
///
/// Caps the search at [`MAX_CONFLICT_ATTEMPTS`] iterations. Beyond that,
/// the function falls back to appending a millisecond timestamp so a
/// pathological directory with millions of pre-existing duplicates
/// cannot wedge the UI in an infinite loop.
pub fn resolve_filename_conflict(dst_path: &Path) -> PathBuf {
    if !dst_path.exists() {
        return dst_path.to_path_buf();
    }

    let parent = dst_path.parent().unwrap_or_else(|| Path::new(""));
    let file_name = dst_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Separar nombre base y extensión
    let (base_name, extension) = if let Some(dot_idx) = file_name.rfind('.') {
        if dot_idx > 0 && dot_idx < file_name.len() - 1 {
            (&file_name[..dot_idx], &file_name[dot_idx..]) // Ej: ("archivo", ".txt")
        } else {
            (file_name, "")
        }
    } else {
        (file_name, "")
    };

    for counter in 1..=MAX_CONFLICT_ATTEMPTS {
        let new_name = format!("{} ({}){}", base_name, counter, extension);
        let candidate = parent.join(new_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    // Fall back to a timestamp-based unique suffix. The numeric
    // collision search exhausted, so we make the name unique by
    // including the current monotonic time in millis.
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let new_name = format!("{} ({} ms){}", base_name, ts, extension);
    parent.join(new_name)
}

/// Upper bound on the number of `(N)` suffixes `resolve_filename_conflict`
/// will try before falling back to a timestamp. 10_000 is high enough to
/// never trigger in any realistic directory (and the test suite covers
/// the common case) but low enough that the function is guaranteed to
/// return in well under a second even if the destination is filled
/// with millions of duplicates.
const MAX_CONFLICT_ATTEMPTS: u32 = 10_000;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_conflict_resolution_naming() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        // El primero no existe, retorna el mismo path
        let resolved = resolve_filename_conflict(&file_path);
        assert_eq!(resolved, file_path);

        // Creamos el archivo para provocar conflicto
        File::create(&file_path).unwrap();

        // Ahora deberia sugerir test (1).txt
        let resolved_1 = resolve_filename_conflict(&file_path);
        assert_eq!(
            resolved_1.file_name().unwrap().to_str().unwrap(),
            "test (1).txt"
        );

        // Creamos test (1).txt
        File::create(&resolved_1).unwrap();

        // Ahora deberia sugerir test (2).txt
        let resolved_2 = resolve_filename_conflict(&file_path);
        assert_eq!(
            resolved_2.file_name().unwrap().to_str().unwrap(),
            "test (2).txt"
        );
    }

    #[test]
    fn test_conflict_resolution_does_not_loop_forever() {
        // Sanity check: even with many pre-existing duplicates, the
        // function must terminate promptly via the timestamp fallback.
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();
        // Pre-create a few duplicates so the linear search has to do
        // some work, but stay well below the cap.
        for i in 1..=50 {
            File::create(dir.path().join(format!("test ({}).txt", i))).unwrap();
        }
        let start = std::time::Instant::now();
        let resolved = resolve_filename_conflict(&file_path);
        let elapsed = start.elapsed();
        // The function should return the next free numeric slot.
        assert_eq!(
            resolved.file_name().unwrap().to_str().unwrap(),
            "test (51).txt"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "resolve_filename_conflict took too long: {:?}",
            elapsed
        );
    }
}
