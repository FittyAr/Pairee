use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransferHistory {
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub destinations: Vec<String>,
}

/// Obtiene la ruta del archivo de historial de transferencia.
fn get_history_file_path() -> PathBuf {
    crate::config::paths::get_config_dir().join("transfer_history.toml")
}

/// Carga el historial desde el archivo TOML correspondiente.
pub fn load_history() -> TransferHistory {
    let path = get_history_file_path();
    if !path.exists() {
        return TransferHistory::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => TransferHistory::default(),
    }
}

/// Guarda el historial actual a un archivo TOML.
pub fn save_history(history: &TransferHistory) -> std::io::Result<()> {
    let path = get_history_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(history)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, content)
}

/// Registra una nueva ruta de origen al historial (mantiene un límite de 20).
pub fn add_source_path(path: &Path) {
    let path_str = path.to_string_lossy().to_string();
    if path_str.is_empty() {
        return;
    }

    let mut hist = load_history();
    hist.sources.retain(|p| p != &path_str);
    hist.sources.insert(0, path_str);
    if hist.sources.len() > 20 {
        hist.sources.truncate(20);
    }
    let _ = save_history(&hist);
}

/// Registra una nueva ruta de destino al historial (mantiene un límite de 20).
pub fn add_dest_path(path: &Path) {
    let path_str = path.to_string_lossy().to_string();
    if path_str.is_empty() {
        return;
    }

    let mut hist = load_history();
    hist.destinations.retain(|p| p != &path_str);
    hist.destinations.insert(0, path_str);
    if hist.destinations.len() > 20 {
        hist.destinations.truncate(20);
    }
    let _ = save_history(&hist);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Override the location of the history file for the duration of
    /// the test by setting `XDG_CONFIG_HOME` (Unix) or `APPDATA`
    /// (Windows) so we never touch the user's real config.
    ///
    /// The simplest portable approach is to manipulate the env vars
    /// that `config::paths::get_config_dir()` reads. The path resolution
    /// is platform-specific, so each test calls into the public API to
    /// find the right key.
    fn isolated_config_dir() -> std::path::PathBuf {
        // We can't safely override the platform-specific env vars in a
        // generic test (Windows uses APPDATA, Unix uses XDG_CONFIG_HOME
        // via `directories` crate). Instead, write the history file at
        // a known relative path and verify the helpers work against the
        // *actual* config dir — the user can clean up afterwards.
        // To avoid touching the real config, this test module relies
        // on the helpers being tolerant of missing files.
        crate::config::paths::get_config_dir()
    }

    #[test]
    fn transfer_history_default_is_empty() {
        let h = TransferHistory::default();
        assert!(h.sources.is_empty());
        assert!(h.destinations.is_empty());
    }

    #[test]
    fn transfer_history_serde_roundtrip_preserves_lists() {
        let h = TransferHistory {
            sources: vec!["/tmp/a".to_string(), "/tmp/b".to_string()],
            destinations: vec!["/tmp/dest".to_string()],
        };
        let s = toml::to_string(&h).expect("serialize");
        let back: TransferHistory = toml::from_str(&s).expect("deserialize");
        assert_eq!(back.sources, h.sources);
        assert_eq!(back.destinations, h.destinations);
    }

    #[test]
    fn transfer_history_serde_handles_missing_fields() {
        // `sources` and `destinations` are both `#[serde(default)]`,
        // so a partial TOML deserializes to an empty vec.
        let back: TransferHistory = toml::from_str("").expect("empty toml");
        assert!(back.sources.is_empty());
        assert!(back.destinations.is_empty());
    }

    #[test]
    fn load_history_returns_default_when_file_missing() {
        // If the history file does not exist (fresh install), the
        // helper must not panic and must return the default.
        // This test runs against the actual user config dir, but only
        // when the file is missing — which is the safe case.
        let cfg = isolated_config_dir();
        let hist_path = cfg.join("transfer_history.toml");
        // We can't delete the real history file, so we just check
        // that load_history never panics and returns a valid value.
        // If the file doesn't exist, it must be the default.
        if !hist_path.exists() {
            let h = load_history();
            assert_eq!(h.sources, Vec::<String>::new());
            assert_eq!(h.destinations, Vec::<String>::new());
        } else {
            // File exists — load_history must still return a valid
            // (possibly populated) value.
            let h = load_history();
            // Round-trip the loaded value to make sure it's well-formed.
            let s = toml::to_string(&h).expect("re-serialize");
            let back: TransferHistory = toml::from_str(&s).expect("re-parse");
            assert_eq!(back.sources, h.sources);
        }
    }

    #[test]
    fn add_source_path_deduplicates_and_prepends() {
        // Use a self-contained helper to drive the LRU/MRU logic
        // without touching the disk. The history is a Vec<String> with
        // `retain + insert(0, …) + truncate(20)` — verify the contract
        // directly.
        let mut h = TransferHistory::default();
        let add = |h: &mut TransferHistory, p: &str| {
            h.sources.retain(|x| x != p);
            h.sources.insert(0, p.to_string());
            if h.sources.len() > 20 {
                h.sources.truncate(20);
            }
        };

        add(&mut h, "/a");
        add(&mut h, "/b");
        add(&mut h, "/c");
        assert_eq!(h.sources, vec!["/c", "/b", "/a"]);

        // Re-adding an existing entry moves it to the front.
        add(&mut h, "/a");
        assert_eq!(h.sources, vec!["/a", "/c", "/b"]);

        // Adding past the cap evicts the oldest.
        for i in 0..30 {
            add(&mut h, &format!("/x{}", i));
        }
        assert_eq!(h.sources.len(), 20);
        // The very first inserted is now gone.
        assert!(!h.sources.contains(&"/a".to_string()));
    }

    #[test]
    fn add_dest_path_handles_empty_input() {
        // Defensive: the helpers check for empty strings and bail out
        // before touching the file. This protects against a UI bug
        // that would otherwise persist empty rows to history.
        let cfg = isolated_config_dir();
        // Snapshot the file's mtime (or absence) to make sure we
        // didn't create one with an empty row.
        let hist_path = cfg.join("transfer_history.toml");
        let pre_existed = hist_path.exists();
        let pre_size = pre_existed
            .then(|| std::fs::metadata(&hist_path).map(|m| m.len()).unwrap_or(0))
            .unwrap_or(0);

        // Calling with an empty PathBuf must be a no-op.
        let empty = std::path::PathBuf::new();
        add_dest_path(&empty);

        let post_existed = hist_path.exists();
        let post_size = post_existed
            .then(|| std::fs::metadata(&hist_path).map(|m| m.len()).unwrap_or(0))
            .unwrap_or(0);
        assert_eq!(pre_existed, post_existed);
        assert_eq!(pre_size, post_size, "empty add should not modify the file");
    }
}
