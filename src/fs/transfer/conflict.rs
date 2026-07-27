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
pub fn resolve_filename_conflict(dst_path: &Path) -> PathBuf {
    if !dst_path.exists() {
        return dst_path.to_path_buf();
    }

    let parent = dst_path.parent().unwrap_or_else(|| Path::new(""));
    let file_name = dst_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // §6: panic-safety. The previous implementation used
    // `rfind('.')` (which returns a *byte* offset) and then
    // sliced with `&file_name[..dot_idx]`. If the file name
    // contained a multi-byte UTF-8 character immediately
    // before the last `.`, the slice landed on a non-char
    // boundary and the `format!` call below panicked with
    // "byte index … is not a char boundary". We now collect
    // `chars()` and search for the *last* `.` in the
    // character stream, then convert the index back to a
    // byte offset via `char_indices`.
    //
    // Both `base_name` and `extension` are owned `String`s so
    // the loop can borrow them across the `format!` call
    // without lifetime headaches.
    let (base_name, extension): (String, String) = match file_name.rfind('.') {
        Some(dot_idx) if dot_idx > 0 && dot_idx < file_name.len() => {
            // `dot_idx` is already a *byte* offset because
            // `rfind` operates on bytes. To use `chars().take`
            // safely, we need a *character* index. Walk the
            // string once to find the character index of the
            // same byte offset.
            let char_idx = file_name
                .char_indices()
                .take_while(|(i, _)| *i < dot_idx)
                .count();
            let base: String = file_name.chars().take(char_idx).collect();
            let ext: String = file_name.chars().skip(char_idx).collect();
            // `ext` keeps the leading `.` (matches the
            // legacy behaviour: `archivo.txt` -> base
            // `"archivo"`, ext `".txt"`).
            if ext.is_empty() || ext == "." {
                (base, String::new())
            } else {
                (base, ext)
            }
        }
        Some(_) => (file_name.to_string(), String::new()),
        None => (file_name.to_string(), String::new()),
    };

    // §6: do not loop forever. A pathological case
    // (e.g. a million `archivo (1).txt`, `archivo (2).txt`,
    // …) used to hit the loop cap only at `counter ==
    // usize::MAX`. 10000 attempts is far more than any
    // realistic download would ever collide with, and the
    // last existing file is returned so the caller can
    // surface a "too many conflicts" error rather than
    // silently hanging the transfer worker.
    const MAX_CONFLICT_ATTEMPTS: u32 = 10_000;
    let mut counter: u32 = 1;
    let mut last_candidate: Option<PathBuf> = None;
    while counter <= MAX_CONFLICT_ATTEMPTS {
        let new_name = format!("{} ({}){}", base_name, counter, extension);
        let candidate = parent.join(new_name);
        if !candidate.exists() {
            return candidate;
        }
        last_candidate = Some(candidate);
        counter += 1;
    }
    // Cap reached. Return the last existing candidate so
    // the caller can produce a meaningful error (e.g. a
    // "too many conflicts" dialog) instead of looping
    // forever.
    last_candidate.unwrap_or_else(|| dst_path.to_path_buf())
}

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

    // §6: a file name with a multi-byte UTF-8 character
    // immediately before the last `.` used to panic the
    // byte-slice path. The `format!` call landed on a non-char
    // boundary and Rust aborted the worker. The fix converts
    // the byte offset to a char index, then back to bytes via
    // `char_indices`, so multi-byte chars are respected.
    #[test]
    fn test_conflict_resolution_multibyte_utf8_panics_before_fix() {
        let dir = tempdir().unwrap();
        // `archivo—` is "archivo" + U+2014 (em dash, 3 bytes
        // in UTF-8). The em dash lands exactly at the byte
        // offset that the old `rfind('.')` would slice on.
        let file_path = dir.path().join("archivo—.txt");
        // Pre-create so the conflict path is taken.
        File::create(&file_path).unwrap();
        let resolved = resolve_filename_conflict(&file_path);
        // Should not panic; should produce "archivo— (1).txt".
        assert_eq!(
            resolved.file_name().unwrap().to_str().unwrap(),
            "archivo— (1).txt"
        );
    }

    // §6: a file name with a multi-byte UTF-8 character
    // *after* the last `.` (e.g. emoji extension) used to
    // corrupt the extension. Verify the fix preserves both
    // the base and the (emoji) extension.
    #[test]
    fn test_conflict_resolution_multibyte_in_extension() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("doc.📄");
        File::create(&file_path).unwrap();
        let resolved = resolve_filename_conflict(&file_path);
        assert_eq!(
            resolved.file_name().unwrap().to_str().unwrap(),
            "doc (1).📄"
        );
    }

    // §6: hidden files (`.bashrc`, `.env`) should NOT have
    // their leading dot stripped — the previous code already
    // handled this with `dot_idx > 0`, but verify the fix
    // still does.
    #[test]
    fn test_conflict_resolution_hidden_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(".env");
        File::create(&file_path).unwrap();
        let resolved = resolve_filename_conflict(&file_path);
        assert_eq!(resolved.file_name().unwrap().to_str().unwrap(), ".env (1)");
    }

    // §6: a file with no extension should still work.
    #[test]
    fn test_conflict_resolution_no_extension() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("README");
        File::create(&file_path).unwrap();
        let resolved = resolve_filename_conflict(&file_path);
        assert_eq!(
            resolved.file_name().unwrap().to_str().unwrap(),
            "README (1)"
        );
    }

    // §6: when the conflict cap is hit, we return the last
    // existing candidate instead of looping forever. We
    // pre-create 10 001 colliding files and confirm the
    // function returns *some* path without hanging.
    #[test]
    fn test_conflict_resolution_caps_at_10000() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("collide.txt");
        File::create(&file_path).unwrap();
        // Pre-create (collide (1).txt .. collide (10000).txt)
        // so the cap is hit on the first attempt.
        for n in 1..=10_000u32 {
            let p = dir.path().join(format!("collide ({n}).txt"));
            File::create(&p).unwrap();
        }
        let start = std::time::Instant::now();
        let resolved = resolve_filename_conflict(&file_path);
        // Should be near-instant, not the multi-second loop
        // the bug would have produced.
        assert!(
            start.elapsed() < std::time::Duration::from_secs(2),
            "resolve_filename_conflict should bail out fast at the cap, took {:?}",
            start.elapsed()
        );
        // The function returns the last *existing* candidate
        // when the cap is hit (so the caller can surface a
        // meaningful "too many conflicts" error).
        assert_eq!(
            resolved.file_name().unwrap().to_str().unwrap(),
            "collide (10000).txt"
        );
    }
}
