use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};

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
