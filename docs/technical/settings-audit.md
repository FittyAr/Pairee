# Settings audit (config dialog vs. runtime)

Audit of every `Settings` / `ConfirmationSettings` field, checking whether
anything outside `src/config/settings/` and the configuration dialog
(`src/app/input_popup/config_dialog/`, `src/ui/popup/config_dialog/`)
actually reads it.

Method: search for `settings.<field>` / `confirmations.<field>` reads in
`src/`. A field that is only toggled by the dialog and serialized to
`config.toml` has no effect on the program.

## Wired up

| Field | Now used by |
| :--- | :--- |
| `save_commands_history`, `save_folders_history`, `save_view_and_edit_history` | `app/app/mod.rs`: history categories that are switched off are neither restored on startup nor written on exit. |
| `editor_tab_size` | Built-in editor (F4) expands tabs to this width; the cursor column follows. |
| `editor_expand_tabs` | Typed `TabExpansion` enum (still reads the old string labels; unknown values fall back to "Do not expand tabs"). `Tab` inserts spaces when expanding; "Convert all" also converts tabs when a file is opened. |
| `editor_auto_indent` | `Enter` copies the leading whitespace of the current line. |
| `editor_show_line_numbers` | Line-number gutter of the editor (default now `true`, which was the previous hard-coded look). |
| `editor_cursor_at_end` | Files open with the cursor on the last line. |
| `editor_lock_editing_readonly`, `editor_warn_opening_readonly` | Read-only files open locked (edits refused, `Shift+F2` saves a copy) or with a notice. |
| `viewer_tab_size` | Internal viewer (F3, text mode) expands tabs to this width. |
| `sorting_collation` | `natural` sorts digit runs numerically (same as "treat digits as numbers"); `linguistic` keeps the previous order. Resolved in `AppState::natural_sort`; the listing code receives only the resulting flag. |
| `left_panel_visible`, `right_panel_visible` | Applied at startup; captured by **Save setup** (Shift+F9) and by `auto_save_setup` on exit. |
| `panel_view_mode`, `sort_field`, `sort_reverse`, `show_long_names` | Same: applied to both panels at startup, captured from the active panel on Save setup. |
| `disable_panel_update_object_count` | Only skips automatic rereads of the same directory; Ctrl+R always rereads. |
| `delete_to_recycle_bin` | The trash backend is the `trash` crate on every platform, with no silent permanent-delete fallback. |

## Removed

Fields that were never read were deleted from `Settings` /
`ConfirmationSettings` together with their translation keys and manual
entries. `Settings` does not use `deny_unknown_fields`, so `config.toml`
files written by older releases that still contain these keys load fine
(covered by `removed_external_editor_keys_are_ignored` in
`src/config/settings/tests.rs`); the keys disappear the next time the
settings are saved.

**External editor (product decision: Pairee edits only with its built-in
editor):** `editor_use_external`, `default_editor`. The shipped file
associations no longer launch `notepad`/`nano` for text files, and untouched
legacy rules of that kind are removed from `associations.toml` on load.

**System:** `use_system_copy_routine`, `copy_files_opened_for_writing`,
`scan_symbolic_links`, `req_admin_use_additional_privileges`.

**Panel:** `right_click_selects_files`, `network_drives_autorefresh`,
`detect_volume_mount_points`, `show_background_screens_number`,
`infopanel_show_power_status`, `infopanel_show_cd_drive_parameters`,
`infopanel_computer_name_format`, `infopanel_user_name_format`,
`file_descriptions_list_names`, `file_descriptions_set_hidden`,
`file_descriptions_update_readonly`, `file_descriptions_position`,
`file_descriptions_update_mode`, `file_descriptions_use_ansi`,
`file_descriptions_save_utf8`, `folder_description_list_names`.

**Interface:** `interface_screen_saver_minutes`,
`interface_show_total_copy_progress`, `interface_show_copying_time`,
`interface_show_total_delete_progress`, `interface_use_ctrl_pgup_change_drive`,
`interface_use_virtual_terminal`, `interface_fullwidth_aware_rendering`,
`interface_cleartype_friendly_redraw`, `interface_console_icon`,
`interface_console_icon_admin_alternate`, `interface_window_title_addons`,
`dialog_history_in_edit_controls`, `dialog_persistent_blocks`,
`dialog_del_removes_blocks`, `dialog_autocomplete`,
`dialog_backspace_deletes_unchanged`, `dialog_mouse_click_outside_closes`,
`menu_left_click_outside`, `menu_right_click_outside`,
`menu_middle_click_outside`, `cmdline_persistent_blocks`,
`cmdline_del_removes_blocks`, `cmdline_autocomplete`, `cmdline_prompt_format`,
`cmdline_use_home_dir`, `autocomplete_show_list`, `autocomplete_modal_mode`,
`autocomplete_append_first`.

**Plugins:** `plugins_manager_oem_support`, `plugins_manager_scan_symlinks`,
`plugins_manager_file_processing`, `plugins_manager_show_standard_association`,
`plugins_manager_even_if_one_found`, `plugins_manager_search_results`,
`plugins_manager_prefix_processing`.

**Editor:** `editor_persistent_blocks`, `editor_cursor_beyond_eol`,
`editor_del_removes_blocks`, `editor_select_found`, `editor_show_scrollbar`,
`editor_show_white_space`, `editor_save_file_position`,
`editor_save_bookmarks`, `editor_allow_editing_opened_writing`,
`editor_autodetect_codepage`, `editor_default_codepage`. (The editor has no
block selection; files are edited as UTF-8 only.)

**Viewer:** `viewer_command`, `viewer_persistent_selection`,
`viewer_show_scrolling_arrows`, `viewer_visible_zero`,
`viewer_save_file_position`, `viewer_save_view_mode`,
`viewer_save_file_codepage`, `viewer_save_wrap_mode`, `viewer_save_bookmarks`,
`viewer_detect_dump_view_mode`, `viewer_max_line_width`,
`viewer_autodetect_codepage`, `viewer_default_codepage` (both reintroduced in
phase G.1, now wired to the viewer's encoding detection).
(`viewer_use_external` stays: the external program comes from the file
association rule.)

**Git:** `git_auto_detect` (repository detection always runs for local
listings).

**Confirmations:** `confirm_overwrite` (the transfer engine decides via
`transfer_conflict_resolution`), `confirm_drag_and_drop`,
`confirm_disconnect_network_drive`, `confirm_delete_subst_disk`,
`confirm_detach_virtual_disk`, `confirm_hotplug_removal`.

**Not in the dialog, never read:** `transfer_engine_enabled`.

## Good next candidates

- `file_descriptions_*` were removed; if `descript.ion` behaviour becomes
  configurable, add typed fields next to `fs/descriptions.rs` instead of
  reviving the old Far-style strings.
- `auto_drop_menu` is read but has no dialog row.
