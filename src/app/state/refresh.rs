use super::AppState;
use crate::fs;

impl AppState {
    /// Refreshes directories inside left and right panels, using full panel settings.
    pub fn refresh_both_panels(&mut self, show_hidden: bool) {
        let left_path = self.left_panel.current_path.clone();
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

            if let Ok(entries) = res {
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

            if let Ok(entries) = res {
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
}
