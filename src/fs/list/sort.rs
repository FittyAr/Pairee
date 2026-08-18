use crate::fs::entry::FileEntry;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmp_natural() {
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
        assert_eq!(
            cmp_natural("File2", "file2", false),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            cmp_natural("File2", "file2", true),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn test_cmp_standard() {
        assert_eq!(
            cmp_standard("file10", "file2", false),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            cmp_standard("File", "file", false),
            std::cmp::Ordering::Equal
        );
        assert_eq!(cmp_standard("File", "file", true), std::cmp::Ordering::Less);
    }
}
