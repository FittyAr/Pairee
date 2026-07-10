use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConflictResolution {
    Ask,
    Overwrite,
    OverwriteOlder,
    Skip,
    Rename,
    KeepBoth,
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

    let mut counter = 1;
    loop {
        let new_name = format!("{} ({}){}", base_name, counter, extension);
        let candidate = parent.join(new_name);
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;

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
        assert_eq!(resolved_1.file_name().unwrap().to_str().unwrap(), "test (1).txt");

        // Creamos test (1).txt
        File::create(&resolved_1).unwrap();

        // Ahora deberia sugerir test (2).txt
        let resolved_2 = resolve_filename_conflict(&file_path);
        assert_eq!(resolved_2.file_name().unwrap().to_str().unwrap(), "test (2).txt");
    }
}
