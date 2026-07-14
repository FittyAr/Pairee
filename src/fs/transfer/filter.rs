use crate::app::state::glob_matches;
use chrono::TimeZone;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterRule {
    Glob(String),
    ExcludeGlob(String),
    SizeMin(u64),
    SizeMax(u64),
    DateNewer(std::time::SystemTime),
    DateOlder(std::time::SystemTime),
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
    /// Soporta filtros de fecha: `newer:2026-01-01` o `older:30d`.
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
            } else if part.starts_with("newer:") {
                if let Some(time) = parse_date(&part[6..]) {
                    rules.push(FilterRule::DateNewer(time));
                }
            } else if part.starts_with("older:") {
                if let Some(time) = parse_date(&part[6..]) {
                    rules.push(FilterRule::DateOlder(time));
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

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let mut has_include_glob = false;
        let mut glob_matched = false;
        let mut file_modified = None;

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
                FilterRule::DateNewer(limit) => {
                    if file_modified.is_none() {
                        file_modified = path.metadata().and_then(|m| m.modified()).ok();
                    }
                    if let Some(mtime) = file_modified {
                        if mtime < *limit {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                FilterRule::DateOlder(limit) => {
                    if file_modified.is_none() {
                        file_modified = path.metadata().and_then(|m| m.modified()).ok();
                    }
                    if let Some(mtime) = file_modified {
                        if mtime > *limit {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
        }

        if has_include_glob { glob_matched } else { true }
    }
}

fn parse_size(size_str: &str) -> Option<u64> {
    let size_str = size_str.trim();
    let num_str: String = size_str
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
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

fn parse_date(date_str: &str) -> Option<std::time::SystemTime> {
    let date_str = date_str.trim();
    if date_str.ends_with('d') || date_str.ends_with('D') {
        let days_str: String = date_str
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let days: u64 = days_str.parse().ok()?;
        let duration = std::time::Duration::from_secs(days * 24 * 60 * 60);
        std::time::SystemTime::now().checked_sub(duration)
    } else {
        let parts: Vec<&str> = date_str.split('-').collect();
        if parts.len() == 3 {
            let year: i32 = parts[0].parse().ok()?;
            let month: u32 = parts[1].parse().ok()?;
            let day: u32 = parts[2].parse().ok()?;
            let nd = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
            let ndt = nd.and_hms_opt(0, 0, 0)?;
            let utc = chrono::Utc.from_local_datetime(&ndt).single()?;
            Some(std::time::SystemTime::from(utc))
        } else {
            None
        }
    }
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

    #[test]
    fn test_date_filters() {
        let temp_path = Path::new("temp_test_date_filter.txt");
        let _ = std::fs::write(temp_path, "test");

        let filter_newer = TransferFilter::parse("newer:2020-01-01");
        // Debería coincidir para un archivo nuevo/modificado recientemente
        assert!(filter_newer.matches(temp_path, 100));

        let filter_relative = TransferFilter::parse("older:5d");
        // No coincidirá si el archivo se modificó hoy
        assert!(!filter_relative.matches(temp_path, 100));

        let _ = std::fs::remove_file(temp_path);
    }
}
