use crate::fs::glob::glob_matches;
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

            if let Some(pattern) = part.strip_prefix('!') {
                rules.push(FilterRule::ExcludeGlob(pattern.to_string()));
            } else if let Some(size_str) = part.strip_prefix('>') {
                if let Some(bytes) = parse_size(size_str) {
                    rules.push(FilterRule::SizeMin(bytes));
                }
            } else if let Some(size_str) = part.strip_prefix('<') {
                if let Some(bytes) = parse_size(size_str) {
                    rules.push(FilterRule::SizeMax(bytes));
                }
            } else if let Some(date_str) = part.strip_prefix("newer:") {
                if let Some(time) = parse_date(date_str) {
                    rules.push(FilterRule::DateNewer(time));
                }
            } else if let Some(date_str) = part.strip_prefix("older:") {
                if let Some(time) = parse_date(date_str) {
                    rules.push(FilterRule::DateOlder(time));
                }
            } else {
                rules.push(FilterRule::Glob(part.to_string()));
            }
        }

        Self { rules }
    }

    /// True when an exclusion (`!pattern`) matches `name`. Folders are only
    /// filtered by exclusions (include patterns and size/date rules apply to
    /// files).
    pub fn excludes(&self, name: &str) -> bool {
        self.rules
            .iter()
            .any(|rule| matches!(rule, FilterRule::ExcludeGlob(pat) if glob_matches(pat, name)))
    }

    /// Comprueba si un archivo cumple las reglas del filtro.
    pub fn matches(
        &self,
        path: &Path,
        file_size: u64,
        file_modified: Option<std::time::SystemTime>,
    ) -> bool {
        if self.rules.is_empty() {
            return true;
        }

        // Lossy, so a name that is not valid Unicode still meets the
        // exclusions (a filtered delete must never take it by accident).
        let file_name = crate::fs::file_name_lossy(path);
        let file_name = file_name.as_str();
        if self.excludes(file_name) {
            return false;
        }

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
                FilterRule::ExcludeGlob(_) => {} // checked by `excludes` above
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
                // The time comes from the entry's own filesystem (a remote
                // or archive path means nothing to the local disk).
                FilterRule::DateNewer(limit) => {
                    if let Some(mtime) = file_modified {
                        if mtime < *limit {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                FilterRule::DateOlder(limit) => {
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
        assert!(filter.matches(Path::new("image.jpg"), 5 * 1024 * 1024, None));

        // Excluido por tamaño demasiado pequeño
        assert!(!filter.matches(Path::new("image.jpg"), 500 * 1024, None));

        // Excluido por glob de exclusion
        assert!(!filter.matches(Path::new("temp_image.png"), 2 * 1024 * 1024, None));

        // Excluido porque no es ni jpg ni png
        assert!(!filter.matches(Path::new("document.pdf"), 2 * 1024 * 1024, None));
    }

    #[test]
    fn test_date_filters() {
        // The time is the entry's (a remote path has no local metadata).
        let path = Path::new("/remote/only/report.txt");
        let now = Some(std::time::SystemTime::now());

        let filter_newer = TransferFilter::parse("newer:2020-01-01");
        assert!(filter_newer.matches(path, 100, now));
        assert!(!filter_newer.matches(path, 100, None));

        let filter_relative = TransferFilter::parse("older:5d");
        assert!(!filter_relative.matches(path, 100, now));
    }

    /// A name that is not valid Unicode still meets the exclusions.
    #[cfg(unix)]
    #[test]
    fn exclusions_apply_to_names_that_are_not_unicode() {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::OsStr::from_bytes(b"caf\xe9.keep");
        let filter = TransferFilter::parse("!*.keep");
        assert!(!filter.matches(Path::new(name), 1, None));
    }
}
