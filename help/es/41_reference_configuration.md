# 📖 Referencia: Configuración

> **Cuadrante: REFERENCE** — *orientado a información. Consultá.*

Esta página lista cada campo del diálogo **Configuration** de Pairee
(`F9` → `Options` → `Configuration`). Cada pestaña es una sección.

> Los settings también se pueden editar directamente en `settings.toml`
> en tu carpeta de config. Los nombres de campo matchean
> exactamente las claves TOML.

---

## Pestaña 0: System

### Operaciones de archivos

| Campo | Default | Efecto |
| --- | --- | --- |
| `delete_to_recycle_bin` | `true` | Cuando es `true`, los borrados van a la papelera del OS. Cuando es `false`, los borrados son permanentes. |
| `use_system_copy_routine` | `false` | Cuando es `true`, las copias y movidas usan la API de copia del OS. Cuando es `false`, Pairee usa sus propios streams de workers async (que soportan políticas custom de overwrite / skip). |
| `copy_files_opened_for_writing` | `false` | Cuando es `false`, los archivos actualmente lockeados por otro proceso se saltean. |
| `scan_symbolic_links` | `true` | Recorre symlinks durante operaciones recursivas. |

### Preservación de historial

| Campo | Default | Efecto |
| --- | --- | --- |
| `save_commands_history` | `true` | Persistir el historial de línea de comandos entre sesiones. |
| `save_folders_history` | `true` | Persistir la lista de carpetas visitadas. |
| `save_view_and_edit_history` | `true` | Persistir los archivos abiertos con `F3` / `F4`. |

### Ambiente y registry

| Campo | Default | Efecto |
| --- | --- | --- |
| `use_windows_registered_types` | `false` | (Solo Windows) Leer asociaciones y descripciones desde el registry. |
| `automatic_update_env_variables` | `true` | Re-leer variables de ambiente en cada comando para que se tomen cambios externos. |

### Permisos y elevación

| Campo | Default | Efecto |
| --- | --- | --- |
| `req_admin_modification` | `false` | Cuando un write/rename pega un error de permisos, pedir elevación (UAC en Windows, `sudo` en Unix). |
| `req_admin_reading` | `false` | Igual que arriba, para errores de lectura. |
| `req_admin_use_additional_privileges` | `false` | Usar el helper elevado para acciones avanzadas. |

### Ordenamiento

| Campo | Default | Efecto |
| --- | --- | --- |
| `sorting_collation` | `"natural"` | `"natural"` (lingüístico) o `"binary"` (comparación de bytes cruda). |
| `treat_digits_as_numbers` | `true` | Ordenar `file2` antes que `file10` (orden natural). |
| `case_sensitive_sort` | `false` | Cuando es `true`, mayúsculas y minúsculas se ordenan por separado. |
| `auto_save_setup` | `true` | Persistir settings al salir. |

---

## Pestaña 1: Panel

### Display & selection

| Campo | Default | Efecto |
| --- | --- | --- |
| `show_hidden_and_system_files` | `false` | Mostrar dotfiles y archivos ocultos del sistema. |
| `highlight_files` | `true` | Colorear archivos por extensión. |
| `select_folders` | `true` | Incluir directorios cuando se tagea con `+` / `-` por glob. |
| `right_click_selects_files` | `false` | El click derecho tagea en vez de abrir un menú contextual. |

### Ordenamiento

| Campo | Default | Efecto |
| --- | --- | --- |
| `sort_folder_names_by_extension` | `false` | Tratar a los directorios como si tuvieran extensión para ordenar. |
| `sort_reverse` | `false` | Invertir el orden actual. |
| `show_sort_mode_letter` | `true` | Mostrar una letra (ej. `n` para Name) en la línea de status. |

### Updates & information

| Campo | Default | Efecto |
| --- | --- | --- |
| `disable_panel_update_object_count` | `false` | Throttling de conteo de items en carpetas muy grandes. |
| `network_drives_autorefresh` | `false` | Vigilar cambios en paths de red montados. |
| `detect_volume_mount_points` | `true` | Resolver cambios de mount de volúmenes en Windows. |
| `show_files_total_information` | `true` | Renderizar conteo total de archivos y bytes al fondo. |
| `show_free_size` | `true` | Mostrar espacio libre del drive actual en el header del panel. |

### Apariencia

| Campo | Default | Efecto |
| --- | --- | --- |
| `show_column_titles` | `true` | Renderizar los headers de columna (Name, Size, Date). |
| `show_status_line` | `true` | Mostrar el conteo de selección y la línea de info. |
| `show_scrollbar` | `true` | Renderizar scrollbars verticales en el panel. |
| `show_background_screens_number` | `true` | Renderizar el conteo de pantallas de fondo. |
| `show_dotdot_in_root_folders` | `false` | Renderizar `..` incluso en directorios root. |

### Info panel & descriptions

| Campo | Default | Efecto |
| --- | --- | --- |
| `computer_user_name_formats` | `"{host} as {user}"` | Tokens usados por el Info panel. |
| `descript_ion` settings | (block) | Nombres de listas (`Descript.ion`), flag hidden, color ANSI, UTF-8, modo de updates. |

---

## Pestaña 2: Interface

| Campo | Default | Efecto |
| --- | --- | --- |
| `clock` | `true` | Mostrar el widget de reloj en vivo. |
| `mouse_support` | `true` | Habilitar navegación, clicks y scroll con mouse. |
| `show_bottom_fkeys_bar` | `true` | Mostrar la barra de hints F1–F12 al fondo. |
| `always_show_menu_bar` | `false` | Mantener visible el menú de arriba. |
| `screen_saver_minutes` | `0` | Blanquear la pantalla después de N minutos de inactividad (`0` = off). |
| `show_total_copy_progress` | `true` | Mostrar progreso agregado y ETA durante copias en bulk. |
| `show_total_delete_progress` | `true` | Mostrar progreso durante borrados en bulk. |
| `use_ctrl_pgup_to_change_drive` | `false` | Usar `Ctrl+PgUp` / `Ctrl+PgDn` para cambiar de drive. |
| `use_virtual_terminal` | `true` | (Windows) Habilitar procesamiento VT. |
| `cleartype_friendly_redraw` | `false` | Workaround para glitches de redraw con ClearType. |
| `window_title_format` | `"Pairee — %Platform — %Path"` | Tokens de la barra de título. |
| `enable_yazi_workflow` | `false` | `s` abre Sort, `v` abre View (solo cuando la línea de comandos está vacía). |

---

## Pestaña 3: Confirmations

| Campo | Default | Efecto |
| --- | --- | --- |
| `confirm_copy` | `true` | Prompt antes de copiar. |
| `confirm_move` | `true` | Prompt antes de mover. |
| `confirm_overwrite` | `true` | Prompt antes de sobreescribir. |
| `confirm_drag_and_drop` | `true` | Prompt antes de acciones de drag-and-drop con mouse. |
| `confirm_delete` | `true` | Prompt antes de borrar items. |
| `confirm_delete_non_empty_folders` | `true` | Prompt extra para carpetas no vacías. |
| `confirm_interrupt_operation` | `true` | Prompt antes de cancelar un job de background. |
| `confirm_disconnect_network_drive` | `true` | Prompt antes de desconectar un mount de red. |
| `confirm_detach_virtual_disk` | `true` | Prompt antes de detachar un disco virtual. |
| `confirm_reload_edited_file` | `true` | Prompt antes de recargar un buffer modificado externamente. |
| `confirm_clear_history_list` | `true` | Prompt antes de limpiar una lista de historial. |
| `confirm_exit` | `false` | Prompt antes de salir de Pairee. |

---

## Pestaña 4: Language & Plugins

### Language

| Campo | Default | Efecto |
| --- | --- | --- |
| `language` | (auto-detect) | Código de idioma activo (ej. `en`, `es`). Pairee toma el primer TOML disponible bajo `lang/`. |

### Plugins

| Campo | Default | Efecto |
| --- | --- | --- |
| `plugins_oem_support` | `false` | Convertir output de plugins OEM-encoded (CP437, CP850) a UTF-8. |
| `plugins_scan_symlinks` | `true` | Seguir symlinks cuando se escanea el directorio de plugins. |
| `plugins_file_processing` | `true` | Delegar apertura / procesamiento de archivos a plugins registrados (ej. navegar archivos como carpetas). |
| `plugins_show_standard_association` | `true` | Mostrar la app default del OS junto a los handlers de plugins. |
| `plugins_show_single_handler` | `false` | Aunque haya un solo plugin que pueda manejar el archivo, mostrar el picker. |
| `plugins_search_results` | `true` | Permitir que los plugins intercepten resultados de búsqueda avanzados. |
| `plugins_prefix_processing` | `true` | Reconocer prefijos de comando como `ftp:host` o `arc:path` para invocar un plugin desde la línea de comandos. |
| `plugins_developer_mode` | `false` | Mostrar la pestaña **Developer Tools** en el Plugin Manager. |
| `plugins_dev_dir` | (config-específico) | Directorio escaneado para plugins en desarrollo. |

---

## Pestaña 5: Editor / Viewer

| Campo | Default | Efecto |
| --- | --- | --- |
| `use_external_editor` | `false` | Delegar `F4` a un comando externo. |
| `editor_command` | (vacío) | Plantilla, ej. `nano %f` o `code --wait %f`. |
| `use_external_viewer` | `false` | Delegar `F3` a un comando externo. |
| `viewer_command` | (vacío) | Plantilla, ej. `less -R %f`. |
| `editor_tab_size` | `4` | Cantidad de espacios por tab. |
| `editor_expand_tabs` | `false` | Insertar espacios en lugar de un caracter de tab. |
| `editor_persistent_blocks` | `false` | Mantener la selección después de mover el cursor. |
| `editor_del_removes_blocks` | `false` | `Del` borra el bloque seleccionado. |
| `editor_cursor_beyond_eol` | `false` | Permitir el caret después del final de línea. |
| `editor_show_line_numbers` | `true` | Renderizar números de línea. |
| `editor_show_whitespace` | `false` | Renderizar marcadores de whitespace. |
| `editor_show_scrollbar` | `true` | Renderizar el scrollbar del editor. |

---

## Pestaña 6: Colors

| Campo | Default | Efecto |
| --- | --- | --- |
| `theme` | `"slate"` | Tema bundled (`"slate"` o `"classic_blue"`) o un nombre de archivo `.toml` custom. |
| `color_groups` | (block) | Overrides de color por slot (mirá [`42_reference_themes`](42_reference_themes.md)). |
| `highlight_rules` | (block) | Overrides de color por extensión. |

---

## Pestaña 7: Git

| Campo | Default | Efecto |
| --- | --- | --- |
| `git_enabled` | `true` | Switch maestro para el dashboard Git. |
| `git_auto_detect` | `true` | Subir el árbol para encontrar la raíz del repo mientras navegás. |
| `git_author_name` | (vacío) | Override de `user.name` para esta sesión. Vacío = usa la git config del sistema. |
| `git_author_email` | (vacío) | Override de `user.email` para esta sesión. |
| `git_max_log_entries` | `200` | Limita cuántos commits muestra la pestaña Log. |

---

## Claves directas de `settings.toml`

Estos campos no están expuestos en el diálogo; editalos a mano:

| Campo | Default | Efecto |
| --- | --- | --- |
| `auto_update_check` | `true` | Consultar GitHub Releases al arrancar. |
| `dismissed_update_version` | (vacío) | El tag de release que el usuario descartó. Limpiá para re-habilitar notificaciones. |
| `keymap` | `"norton"` | Stem del filename del preset activo: `"norton"`, `"neovim"`, `"vscode"`, o un nombre custom. |
| `default_user_language` | `"en"` | Idioma preferido cuando el idioma del sistema no está soportado. |
| `transfer_panel_default_view` | `"progress"` | Pestaña inicial del Transfer Panel (`"progress"` o `"history"`). |
| `auto_drop_menu` | `false` | Cuando `F9` abre el menú, el primer item se auto-resalta. |
| `transfer_engine` | `"async"` | `"async"` (default) o `"direct"` (usa copia del sistema). |
| `yazi_workflow` | `false` | Igual que `enable_yazi_workflow` (alias). |
| `secure_mode` | `false` | Cuando es `true`, los plugins no pueden usar una blacklist de 27 comandos (mirá [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md)). |

---

## A dónde ir ahora

- Esquema TOML de temas: [`42_reference_themes`](42_reference_themes.md)
- Enum de acciones: [`43_reference_actions`](43_reference_actions.md)
