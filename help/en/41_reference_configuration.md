# 📖 Reference: Configuration Settings

> **Quadrant: REFERENCE** — *information-oriented. Look things up.*

This page lists every field in Pairee's **Configuration** dialog
(`F9` → `Options` → `Configuration`). Each tab is a section.

> Settings can also be edited directly in `settings.toml` in your
> config folder. Field names match the TOML keys exactly.

---

## Tab 0: System

### File operations

| Field | Default | Effect |
| --- | --- | --- |
| `delete_to_recycle_bin` | `true` | When `true`, deletes go to the OS Recycle Bin. When `false`, deletes are permanent. |
| `use_system_copy_routine` | `false` | When `true`, copies and moves use the OS copy API. When `false`, Pairee uses its own async worker streams (which support custom overwrite / skip policies). |
| `copy_files_opened_for_writing` | `false` | When `false`, files currently locked by another process are skipped. |
| `scan_symbolic_links` | `true` | Traverse symbolic links during recursive operations. |

### History preservation

| Field | Default | Effect |
| --- | --- | --- |
| `save_commands_history` | `true` | Persist command-line history across sessions. |
| `save_folders_history` | `true` | Persist visited folder list. |
| `save_view_and_edit_history` | `true` | Persist files opened with `F3` / `F4`. |

### Environment & registry

| Field | Default | Effect |
| --- | --- | --- |
| `use_windows_registered_types` | `false` | (Windows only) Read file associations and descriptions from the registry. |
| `automatic_update_env_variables` | `true` | Re-read environment variables on each command so external changes are picked up. |

### Permissions & elevation

| Field | Default | Effect |
| --- | --- | --- |
| `req_admin_modification` | `false` | When a write/rename hits a permission error, prompt for elevation (UAC on Windows, `sudo` on Unix). |
| `req_admin_reading` | `false` | Same as above, for read errors. |
| `req_admin_use_additional_privileges` | `false` | Use the elevated helper for advanced actions. |

### Sorting

| Field | Default | Effect |
| --- | --- | --- |
| `sorting_collation` | `"natural"` | `"natural"` (linguistic) or `"binary"` (raw byte comparison). |
| `treat_digits_as_numbers` | `true` | Sort `file2` before `file10` (natural sort). |
| `case_sensitive_sort` | `false` | When `true`, uppercase and lowercase sort separately. |
| `auto_save_setup` | `true` | Persist settings on exit. |

---

## Tab 1: Panel

### Display & selection

| Field | Default | Effect |
| --- | --- | --- |
| `show_hidden_and_system_files` | `false` | Show dotfiles and system hidden files. |
| `highlight_files` | `true` | Colour files by extension. |
| `select_folders` | `true` | Include directories when using `+` / `-` glob tagging. |
| `right_click_selects_files` | `false` | Right-click tags instead of opening a context menu. |

### Sorting

| Field | Default | Effect |
| --- | --- | --- |
| `sort_folder_names_by_extension` | `false` | Treat directories as having an extension for sorting. |
| `sort_reverse` | `false` | Reverse the current sort order. |
| `show_sort_mode_letter` | `true` | Show a single letter (e.g. `n` for Name) in the status line. |

### Updates & information

| Field | Default | Effect |
| --- | --- | --- |
| `disable_panel_update_object_count` | `false` | Throttle item counts on very large folders. |
| `network_drives_autorefresh` | `false` | Watch mounted network paths for changes. |
| `detect_volume_mount_points` | `true` | Resolve volume mount changes on Windows. |
| `show_files_total_information` | `true` | Render total file count and bytes at the bottom. |
| `show_free_size` | `true` | Show free space on the current drive in the panel header. |

### Appearance

| Field | Default | Effect |
| --- | --- | --- |
| `show_column_titles` | `true` | Render the column headers (Name, Size, Date). |
| `show_status_line` | `true` | Show the selection count and info line. |
| `show_scrollbar` | `true` | Render vertical scrollbars in the panel. |
| `show_background_screens_number` | `true` | Render the count of background screens. |
| `show_dotdot_in_root_folders` | `false` | Render `..` even in root directories. |

### Info panel & descriptions

| Field | Default | Effect |
| --- | --- | --- |
| `computer_user_name_formats` | `"{host} as {user}"` | Tokens used by the Info panel. |
| `descript_ion` settings | (block) | List names (`Descript.ion`), hidden flag, ANSI colour, UTF-8, updates mode. |

---

## Tab 2: Interface

| Field | Default | Effect |
| --- | --- | --- |
| `clock` | `true` | Show the live clock widget. |
| `mouse_support` | `true` | Enable mouse navigation, clicks, and scrolling. |
| `show_bottom_fkeys_bar` | `true` | Show the F1–F12 hint bar at the bottom. |
| `always_show_menu_bar` | `false` | Keep the top menu visible. |
| `screen_saver_minutes` | `0` | Blank the screen after N minutes idle (`0` = off). |
| `show_total_copy_progress` | `true` | Show aggregated progress and ETA during bulk copies. |
| `show_total_delete_progress` | `true` | Show progress during bulk deletes. |
| `use_ctrl_pgup_to_change_drive` | `false` | Use `Ctrl+PgUp` / `Ctrl+PgDn` to change drives. |
| `use_virtual_terminal` | `true` | (Windows) Enable VT processing. |
| `cleartype_friendly_redraw` | `false` | Work around ClearType redraw glitches. |
| `window_title_format` | `"Pairee — %Platform — %Path"` | Title bar tokens. |
| `enable_yazi_workflow` | `false` | `s` opens Sort, `v` opens View (only when command line is empty). |

---

## Tab 3: Confirmations

| Field | Default | Effect |
| --- | --- | --- |
| `confirm_copy` | `true` | Prompt before copy. |
| `confirm_move` | `true` | Prompt before move. |
| `confirm_overwrite` | `true` | Prompt before overwriting. |
| `confirm_drag_and_drop` | `true` | Prompt before mouse drag-and-drop actions. |
| `confirm_delete` | `true` | Prompt before deleting items. |
| `confirm_delete_non_empty_folders` | `true` | Extra prompt for non-empty folders. |
| `confirm_interrupt_operation` | `true` | Prompt before cancelling a background job. |
| `confirm_disconnect_network_drive` | `true` | Prompt before disconnecting a network mount. |
| `confirm_detach_virtual_disk` | `true` | Prompt before detaching a virtual disk. |
| `confirm_reload_edited_file` | `true` | Prompt before reloading an externally modified buffer. |
| `confirm_clear_history_list` | `true` | Prompt before wiping a history list. |
| `confirm_exit` | `false` | Prompt before quitting Pairee. |

---

## Tab 4: Language & Plugins

### Language

| Field | Default | Effect |
| --- | --- | --- |
| `language` | (auto-detect) | Active language code (e.g. `en`, `es`). Pairee picks the first available TOML under `lang/`. |

### Plugins

| Field | Default | Effect |
| --- | --- | --- |
| `plugins_oem_support` | `false` | Convert OEM-encoded plugin output (CP437, CP850) to UTF-8. |
| `plugins_scan_symlinks` | `true` | Follow symlinks when scanning the plugin directory. |
| `plugins_file_processing` | `true` | Delegate file open / process to registered plugins (e.g. browse archives as folders). |
| `plugins_show_standard_association` | `true` | Show the OS default app alongside plugin handlers. |
| `plugins_show_single_handler` | `false` | Even if only one plugin can handle the file, show the picker. |
| `plugins_search_results` | `true` | Allow plugins to intercept advanced search results. |
| `plugins_prefix_processing` | `true` | Recognise command prefixes like `ftp:host` or `arc:path` to invoke a plugin from the command line. |
| `plugins_developer_mode` | `false` | Show the **Developer Tools** tab in the Plugin Manager. |
| `plugins_dev_dir` | (config-specific) | Directory scanned for in-development plugins. |

---

## Tab 5: Editor / Viewer

| Field | Default | Effect |
| --- | --- | --- |
| `use_external_editor` | `false` | Delegate `F4` to an external command. |
| `editor_command` | (empty) | Template, e.g. `nano %f` or `code --wait %f`. |
| `use_external_viewer` | `false` | Delegate `F3` to an external command. |
| `viewer_command` | (empty) | Template, e.g. `less -R %f`. |
| `editor_tab_size` | `4` | Number of spaces per tab. |
| `editor_expand_tabs` | `false` | Insert spaces instead of a tab character. |
| `editor_persistent_blocks` | `false` | Keep selection after cursor move. |
| `editor_del_removes_blocks` | `false` | `Del` removes the selected block. |
| `editor_cursor_beyond_eol` | `false` | Allow the caret past the end of a line. |
| `editor_show_line_numbers` | `true` | Render line numbers. |
| `editor_show_whitespace` | `false` | Render whitespace markers. |
| `editor_show_scrollbar` | `true` | Render the editor scrollbar. |

---

## Tab 6: Colors

| Field | Default | Effect |
| --- | --- | --- |
| `theme` | `"slate"` | Bundled theme (`"slate"` or `"classic_blue"`) or a custom `.toml` filename. |
| `color_groups` | (block) | Per-slot colour overrides (see [`42_reference_themes`](42_reference_themes.md)). |
| `highlight_rules` | (block) | Per-extension colour overrides. |

---

## Tab 7: Git

| Field | Default | Effect |
| --- | --- | --- |
| `git_enabled` | `true` | Master switch for the Git dashboard. |
| `git_auto_detect` | `true` | Walk up the tree to find the repo root as you navigate. |
| `git_author_name` | (empty) | Override `user.name` for this session. Empty = use system git config. |
| `git_author_email` | (empty) | Override `user.email` for this session. |
| `git_max_log_entries` | `200` | Limit how many commits the Log tab shows. |

---

## Direct `settings.toml` keys

These fields are not exposed in the dialog; edit them by hand:

| Field | Default | Effect |
| --- | --- | --- |
| `auto_update_check` | `true` | Query GitHub Releases at launch. |
| `dismissed_update_version` | (empty) | The release tag the user dismissed. Clear to re-enable notifications. |
| `keymap` | `"norton"` | Active preset filename stem: `"norton"`, `"neovim"`, `"vscode"`, or a custom name. |
| `default_user_language` | `"en"` | Preferred language when the system language is unsupported. |
| `transfer_panel_default_view` | `"progress"` | Initial Transfer Panel tab (`"progress"` or `"history"`). |
| `auto_drop_menu` | `false` | When `F9` opens the menu, the first item is auto-highlighted. |
| `transfer_engine` | `"async"` | `"async"` (default) or `"direct"` (use system copy). |
| `yazi_workflow` | `false` | Same as `enable_yazi_workflow` (alias). |
| `secure_mode` | `false` | When `true`, plugins cannot use a 27-command blacklist (see [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md)). |

---

## Where to go next

- Theme TOML schema: [`42_reference_themes`](42_reference_themes.md)
- Action enum: [`43_reference_actions`](43_reference_actions.md)
