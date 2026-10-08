# Settings audit (config dialog vs. runtime)

Audit of every `Settings` / `ConfirmationSettings` field, checking whether
anything outside `src/config/settings/` and the configuration dialog
(`src/app/input_popup/config_dialog/`, `src/ui/popup/config_dialog/`)
actually reads it.

Method: search for `settings.<field>` / `confirmations.<field>` reads in
`src/`. A field that is only toggled by the dialog and serialized to
`config.toml` has no effect on the program.

## Wired up in this change

| Field | Now used by |
| :--- | :--- |
| `save_commands_history`, `save_folders_history`, `save_view_and_edit_history` | `app/app/mod.rs`: history categories that are switched off are neither restored on startup nor written on exit. |
| `editor_tab_size` | Internal editor (F4) expands tabs to this width; the cursor column follows. |
| `viewer_tab_size` | Internal viewer (F3, text mode) expands tabs to this width. |
| `sorting_collation` | `natural` now sorts digit runs numerically (same as "treat digits as numbers"); `linguistic` keeps the previous order. |
| `left_panel_visible`, `right_panel_visible` | Applied at startup; captured by **Save setup** (Shift+F9) and by `auto_save_setup` on exit. |
| `panel_view_mode`, `sort_field`, `sort_reverse`, `show_long_names` | Same: applied to both panels at startup, captured from the active panel on Save setup. The "Reverse sort" dialog row therefore now has an effect (from the next start). |
| `disable_panel_update_object_count` | Already read, but it also blocked loading a *different* directory. It now only skips automatic rereads of the same directory; Ctrl+R always rereads. |
| `delete_to_recycle_bin` | Already read; the trash backend is now the `trash` crate on every platform, with no silent permanent-delete fallback. |

## Hidden from the dialog (still stored in `config.toml`)

These fields are never read. Their dialog rows and handlers were removed so
the dialog no longer offers options that do nothing. The fields stay in
`Settings` (and in existing config files) so they can be implemented later
without a config migration.

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
`plugins_manager_prefix_processing` (plus the static "ArcLite | EMenu |
HlfViewer | NetBox" hint row).

**Editor:** `editor_use_external`, `default_editor`, `editor_expand_tabs`,
`editor_persistent_blocks`, `editor_cursor_beyond_eol`,
`editor_del_removes_blocks`, `editor_select_found`, `editor_auto_indent`,
`editor_cursor_at_end`, `editor_show_scrollbar`, `editor_show_white_space`,
`editor_show_line_numbers`, `editor_save_file_position`,
`editor_save_bookmarks`, `editor_allow_editing_opened_writing`,
`editor_lock_editing_readonly`, `editor_warn_opening_readonly`,
`editor_autodetect_codepage`, `editor_default_codepage`.

**Viewer:** `viewer_command`, `viewer_persistent_selection`,
`viewer_show_scrolling_arrows`, `viewer_visible_zero`,
`viewer_save_file_position`, `viewer_save_view_mode`,
`viewer_save_file_codepage`, `viewer_save_wrap_mode`, `viewer_save_bookmarks`,
`viewer_detect_dump_view_mode`, `viewer_max_line_width`,
`viewer_autodetect_codepage`, `viewer_default_codepage`.
(`viewer_use_external` is read, but the external program comes from the file
association rule, not `viewer_command`.)

**Git:** `git_auto_detect`.

**Confirmations:** `confirm_overwrite`, `confirm_drag_and_drop`,
`confirm_disconnect_network_drive`, `confirm_delete_subst_disk`,
`confirm_detach_virtual_disk`, `confirm_hotplug_removal`.

**Not in the dialog, never read:** `transfer_engine_enabled`.

## Good next candidates

- `editor_use_external` + `default_editor`: F4 could launch the configured
  editor via `exec::execute_external_program`. That helper currently waits
  for Enter after the program exits, which is awkward for an editor, so it
  needs a "no pause" variant first.
- `editor_show_line_numbers`: the editor always draws the 7-column gutter;
  making it optional only touches `ui/popup/editor/widget.rs`.
- `confirm_overwrite`: the transfer engine has its own conflict policy
  (`transfer_conflict_resolution`); decide whether this flag maps onto it.
- `file_descriptions_*`: `fs/descriptions.rs` hardcodes `descript.ion`
  behaviour; these flags map naturally onto it.
- `fs::list::read_directory_ext` still takes an unused `_sorting_collation`
  parameter; collation is now resolved in `AppState::natural_sort`.
- `help/*/configuration_details.md` still describes the hidden options; the
  manuals should be trimmed to match the dialog.
