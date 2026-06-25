use super::{AppState, ActivePanel};
use crate::fs;

impl AppState {
    /// Refreshes directories inside left and right panels, using full panel settings.
    pub fn refresh_both_panels(&mut self, show_hidden: bool) {
        let left_path = self.left_panel.current_path.clone();
        if left_path != self.left_panel.last_path {
            self.left_panel.quick_filter_mask = None;
            self.left_panel.last_path = left_path.clone();
        }
        let left_count = self.left_panel.entries.len();
        let skip_left = self.disable_panel_update_object_count > 0
            && left_count as u32 > self.disable_panel_update_object_count;
        if !skip_left {
            let res = if let Some(client) = &self.left_panel.ssh_conn {
                client.read_directory(
                    &left_path,
                    show_hidden,
                    self.case_sensitive_sort,
                    self.treat_digits_as_numbers,
                    self.left_panel.sort_field,
                    self.left_panel.sort_reverse,
                    self.show_dotdot_in_root_folders,
                )
            } else {
                fs::read_directory_ext(
                    &left_path,
                    show_hidden,
                    self.case_sensitive_sort,
                    self.treat_digits_as_numbers,
                    &self.sorting_collation,
                    self.req_admin_reading,
                    self.left_panel.sort_field,
                    self.left_panel.sort_reverse,
                    self.sort_folder_names_by_extension,
                    self.show_dotdot_in_root_folders,
                )
            };

            if let Ok(mut entries) = res {
                if let Some(ref mask) = self.left_panel.filter_mask {
                    if !mask.is_empty() {
                        entries.retain(|e| e.name == ".." || crate::app::state::glob::glob_matches(mask, &e.name));
                    }
                }
                if let Some(ref qmask) = self.left_panel.quick_filter_mask {
                    if !qmask.is_empty() {
                        entries = partition_entries_by_mask(entries, qmask);
                    }
                }
                self.left_panel.entries = entries;
                if self.left_panel.cursor_index >= self.left_panel.entries.len() {
                    self.left_panel.cursor_index = self.left_panel.entries.len().saturating_sub(1);
                }
            }
            self.free_space_left = if self.left_panel.ssh_conn.is_none() {
                crate::app::sys_helpers::get_free_space(&left_path)
            } else {
                None
            };
        }

        let right_path = self.right_panel.current_path.clone();
        if right_path != self.right_panel.last_path {
            self.right_panel.quick_filter_mask = None;
            self.right_panel.last_path = right_path.clone();
        }
        let right_count = self.right_panel.entries.len();
        let skip_right = self.disable_panel_update_object_count > 0
            && right_count as u32 > self.disable_panel_update_object_count;
        if !skip_right {
            let res = if let Some(client) = &self.right_panel.ssh_conn {
                client.read_directory(
                    &right_path,
                    show_hidden,
                    self.case_sensitive_sort,
                    self.treat_digits_as_numbers,
                    self.right_panel.sort_field,
                    self.right_panel.sort_reverse,
                    self.show_dotdot_in_root_folders,
                )
            } else {
                fs::read_directory_ext(
                    &right_path,
                    show_hidden,
                    self.case_sensitive_sort,
                    self.treat_digits_as_numbers,
                    &self.sorting_collation,
                    self.req_admin_reading,
                    self.right_panel.sort_field,
                    self.right_panel.sort_reverse,
                    self.sort_folder_names_by_extension,
                    self.show_dotdot_in_root_folders,
                )
            };

            if let Ok(mut entries) = res {
                if let Some(ref mask) = self.right_panel.filter_mask {
                    if !mask.is_empty() {
                        entries.retain(|e| e.name == ".." || crate::app::state::glob::glob_matches(mask, &e.name));
                    }
                }
                if let Some(ref qmask) = self.right_panel.quick_filter_mask {
                    if !qmask.is_empty() {
                        entries = partition_entries_by_mask(entries, qmask);
                    }
                }
                self.right_panel.entries = entries;
                if self.right_panel.cursor_index >= self.right_panel.entries.len() {
                    self.right_panel.cursor_index =
                        self.right_panel.entries.len().saturating_sub(1);
                }
            }
            self.free_space_right = if self.right_panel.ssh_conn.is_none() {
                crate::app::sys_helpers::get_free_space(&right_path)
            } else {
                None
            };
        }
    }

    /// Dynamically applies / updates in-memory sorting and filtering on the active panel.
    pub fn update_panel_filter(&mut self, active_panel: ActivePanel, mask: Option<String>) {
        let (panel, case_sensitive, digits_as_numbers, folder_by_ext) = match active_panel {
            ActivePanel::Left => (
                &mut self.left_panel,
                self.case_sensitive_sort,
                self.treat_digits_as_numbers,
                self.sort_folder_names_by_extension,
            ),
            ActivePanel::Right => (
                &mut self.right_panel,
                self.case_sensitive_sort,
                self.treat_digits_as_numbers,
                self.sort_folder_names_by_extension,
            ),
        };

        let prev_selected_path = panel.entries.get(panel.cursor_index).map(|e| e.path.clone());
        let clean_mask = match mask {
            Some(ref m) if m.is_empty() => None,
            m => m,
        };
        panel.quick_filter_mask = clean_mask;

        // 1. Sort the entries back to their original sorted state in-memory
        crate::fs::list::sort_entries(
            &mut panel.entries,
            panel.sort_field,
            panel.sort_reverse,
            case_sensitive,
            digits_as_numbers,
            folder_by_ext,
        );

        // 2. If there is a quick filter mask, partition the entries in-memory!
        if let Some(ref m) = panel.quick_filter_mask {
            if !m.is_empty() {
                panel.entries = partition_entries_by_mask(panel.entries.clone(), m);
            }
        }

        // 3. Restore cursor position if possible
        if let Some(path) = prev_selected_path {
            if let Some(pos) = panel.entries.iter().position(|e| e.path == path) {
                panel.cursor_index = pos;
            } else {
                panel.cursor_index = 0;
            }
        } else {
            panel.cursor_index = 0;
        }
    }
}

fn partition_entries_by_mask(entries: Vec<crate::fs::FileEntry>, mask: &str) -> Vec<crate::fs::FileEntry> {
    let mask_lower = mask.to_lowercase();
    let mut matching = Vec::new();
    let mut non_matching = Vec::new();

    let mut dotdot_entry = None;

    for entry in entries {
        if entry.name == ".." {
            dotdot_entry = Some(entry);
        } else if entry.name.to_lowercase().contains(&mask_lower) {
            matching.push(entry);
        } else {
            non_matching.push(entry);
        }
    }

    let mut result = Vec::new();
    if let Some(dd) = dotdot_entry {
        result.push(dd);
    }
    result.extend(matching);
    result.extend(non_matching);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::FileEntry;
    use std::path::PathBuf;

    #[test]
    fn test_partition_entries_by_mask() {
        let entries = vec![
            FileEntry {
                name: "..".to_string(),
                path: PathBuf::from("/"),
                is_dir: true,
                is_symlink: false,
                size: 0,
                modified: None,
            },
            FileEntry {
                name: "rust_code.rs".to_string(),
                path: PathBuf::from("/rust_code.rs"),
                is_dir: false,
                is_symlink: false,
                size: 100,
                modified: None,
            },
            FileEntry {
                name: "other_file.txt".to_string(),
                path: PathBuf::from("/other_file.txt"),
                is_dir: false,
                is_symlink: false,
                size: 200,
                modified: None,
            },
            FileEntry {
                name: "rust_dir".to_string(),
                path: PathBuf::from("/rust_dir"),
                is_dir: true,
                is_symlink: false,
                size: 0,
                modified: None,
            },
        ];

        let partitioned = partition_entries_by_mask(entries, "rust");

        // The first element should always be ".."
        assert_eq!(partitioned[0].name, "..");
        // The next elements should be matching ones
        assert_eq!(partitioned[1].name, "rust_code.rs");
        assert_eq!(partitioned[2].name, "rust_dir");
        // The last element should be non-matching
        assert_eq!(partitioned[3].name, "other_file.txt");
    }
}
