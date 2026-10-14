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

/// The key a listing is sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    #[default]
    Name,
    Extension,
    Size,
    /// Last write time.
    Date,
    Unsorted,
    /// Creation time.
    Created,
    /// Last access time.
    Accessed,
    /// Owner name ([`SortField::needs_owner`]).
    Owner,
    /// `descript.ion` description ([`SortField::needs_description`]).
    Description,
}

impl SortField {
    pub const ALL: [Self; 9] = [
        Self::Name,
        Self::Extension,
        Self::Size,
        Self::Date,
        Self::Unsorted,
        Self::Created,
        Self::Accessed,
        Self::Owner,
        Self::Description,
    ];

    /// The listing must fill in `FileEntry::owner` to sort by this.
    pub fn needs_owner(self) -> bool {
        self == Self::Owner
    }

    /// The listing must fill in `FileEntry::description` to sort by this.
    pub fn needs_description(self) -> bool {
        self == Self::Description
    }
}

/// Seconds since the epoch, for comparing times (unknown first).
fn time_key(time: Option<std::time::SystemTime>) -> Option<u64> {
    time.map(|t| {
        t.duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    })
}

pub fn sort_entries(
    entries: &mut Vec<FileEntry>,
    sort_field: SortField,
    sort_reverse: bool,
    case_sensitive_sort: bool,
    treat_digits_as_numbers: bool,
    sort_folder_names_by_extension: bool,
) {
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

            let by_name = || {
                if treat_digits_as_numbers {
                    cmp_natural(&a.name, &b.name, case_sensitive_sort)
                } else {
                    cmp_standard(&a.name, &b.name, case_sensitive_sort)
                }
            };
            let by_text = |x: &Option<String>, y: &Option<String>| {
                let (x, y) = (x.as_deref().unwrap_or(""), y.as_deref().unwrap_or(""));
                cmp_standard(x, y, case_sensitive_sort).then_with(by_name)
            };
            let name_ord = match sort_field {
                SortField::Name => by_name(),
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
                SortField::Date => time_key(a.modified).cmp(&time_key(b.modified)),
                SortField::Created => time_key(a.created)
                    .cmp(&time_key(b.created))
                    .then_with(by_name),
                SortField::Accessed => time_key(a.accessed)
                    .cmp(&time_key(b.accessed))
                    .then_with(by_name),
                SortField::Owner => by_text(&a.owner, &b.owner),
                SortField::Description => by_text(&a.description, &b.description),
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

    fn entry(name: &str) -> FileEntry {
        FileEntry {
            name: name.into(),
            ..FileEntry::default()
        }
    }

    fn sorted(mut entries: Vec<FileEntry>, field: SortField) -> Vec<String> {
        sort_entries(&mut entries, field, false, false, false, false);
        entries.into_iter().map(|e| e.name).collect()
    }

    #[test]
    fn creation_access_owner_and_description_sort_by_their_own_field() {
        use std::time::{Duration, UNIX_EPOCH};
        let at = |secs| Some(UNIX_EPOCH + Duration::from_secs(secs));
        let mut a = entry("a");
        let mut b = entry("b");
        let mut c = entry("c");
        // Write times say a, b, c; every other field says otherwise.
        (a.modified, b.modified, c.modified) = (at(1), at(2), at(3));
        (a.created, b.created, c.created) = (at(30), at(10), at(20));
        (a.accessed, b.accessed, c.accessed) = (at(20), at(30), at(10));
        a.owner = Some("zoe".into());
        b.owner = Some("adam".into());
        c.owner = Some("adam".into());
        a.description = Some("beta".into());
        c.description = Some("alpha".into());
        let all = vec![a, b, c];
        assert_eq!(sorted(all.clone(), SortField::Created), ["b", "c", "a"]);
        assert_eq!(sorted(all.clone(), SortField::Accessed), ["c", "a", "b"]);
        assert_eq!(
            sorted(all.clone(), SortField::Owner),
            ["b", "c", "a"],
            "ties by name"
        );
        assert_eq!(
            sorted(all, SortField::Description),
            ["b", "c", "a"],
            "no description first"
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
