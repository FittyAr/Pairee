use std::path::Path;
use crate::app::state::glob_matches;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterRule {
    Glob(String),
    ExcludeGlob(String),
    SizeMin(u64),
    SizeMax(u64),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransferFilter {
    pub rules: Vec<FilterRule>,
}

impl TransferFilter {
    /// Parsea una cadena de filtros a un TransferFilter.
    /// Soporta múltiples patrones separados por `;`.
    /// Si empieza por `!` es exclusión.
    /// Soporta filtros de tamaño si se especifica `>10MB` o `<100KB`.
    pub fn parse(filter_str: &str) -> Self {
        let mut rules = Vec::new();
        for part in filter_str.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            if part.starts_with('!') {
                let pattern = part[1..].to_string();
                rules.push(FilterRule::ExcludeGlob(pattern));
            } else if part.starts_with('>') {
                if let Some(bytes) = parse_size(&part[1..]) {
                    rules.push(FilterRule::SizeMin(bytes));
                }
            } else if part.starts_with('<') {
                if let Some(bytes) = parse_size(&part[1..]) {
                    rules.push(FilterRule::SizeMax(bytes));
                }
            } else {
                rules.push(FilterRule::Glob(part.to_string()));
            }
        }

        Self { rules }
    }

    /// Comprueba si un archivo cumple las reglas del filtro.
    pub fn matches(&self, path: &Path, file_size: u64) -> bool {
        if self.rules.is_empty() {
            return true;
        }

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let mut has_include_glob = false;
        let mut glob_matched = false;

        for rule in &self.rules {
            match rule {
                FilterRule::Glob(pat) => {
                    has_include_glob = true;
                    if glob_matches(pat, file_name) {
                        glob_matched = true;
                    }
                }
                FilterRule::ExcludeGlob(pat) => {
                    if glob_matches(pat, file_name) {
                        return false; // Exclusión directa
                    }
                }
                FilterRule::SizeMin(min_size) => {
                    if file_size < *min_size {
                        return false;
                    }
                }
                FilterRule::SizeMax(max_size) => {
                    if file_size > *max_size {
                        return false;
                    }
                }
            }
        }

        if has_include_glob {
            glob_matched
        } else {
            true
        }
    }
}

fn parse_size(size_str: &str) -> Option<u64> {
    let size_str = size_str.trim();
    let num_str: String = size_str.chars().take_while(|c| c.is_ascii_digit()).collect();
    let num: u64 = num_str.parse().ok()?;
    
    let unit = &size_str[num_str.len()..].trim().to_uppercase();
    let multiplier = match unit.as_str() {
        "KB" | "K" => 1024,
        "MB" | "M" => 1024 * 1024,
        "GB" | "G" => 1024 * 1024 * 1024,
        _ => 1,
    };

    Some(num * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_parsing_and_matching() {
        let filter = TransferFilter::parse("*.jpg;*.png;!temp*;>1MB;<50MB");
        
        // Incluido por glob y dentro de rango de tamaño
        assert!(filter.matches(Path::new("image.jpg"), 5 * 1024 * 1024));
        
        // Excluido por tamaño demasiado pequeño
        assert!(!filter.matches(Path::new("image.jpg"), 500 * 1024));
        
        // Excluido por glob de exclusion
        assert!(!filter.matches(Path::new("temp_image.png"), 2 * 1024 * 1024));
        
        // Excluido porque no es ni jpg ni png
        assert!(!filter.matches(Path::new("document.pdf"), 2 * 1024 * 1024));
    }
}
