# 📖 Referencia: Atajos de Teclado

> **Cuadrante: REFERENCE** — *orientado a información. Consultá.*

Esta página lista cada keybinding en cada preset bundled. Para el
*cómo* cambiar de preset, mirá
[`26_howto_keymaps_user_menu`](26_howto_keymaps_user_menu.md). Para la
lista completa de *acciones* (la segunda columna), mirá
[`43_reference_actions`](43_reference_actions.md).

> **Convenciones**
> - `+` significa "y" (`Ctrl+Shift+S` = mantener Ctrl, mantener Shift,
>   apretar S).
> - `,` significa "binding alternativo" (ej. `F2` e `Insert` disparan
>   `select_item` en el preset Neovim).
> - La primera columna es el preset; la misma acción puede tener
>   teclas distintas en distintos presets.

---

## 1. Navegación

| Acción | Norton | Neovim | VSCode |
| --- | --- | --- | --- |
| Mover cursor arriba | `Up` | `k`, `Up` | `Up` |
| Mover cursor abajo | `Down` | `j`, `Down` | `Down` |
| Página arriba | `PageUp` | `Ctrl+u` | `PageUp` |
| Página abajo | `PageDown` | `Ctrl+d` | `PageDown` |
| Ir al tope | `Home` | `g`, `Home` | `Home` |
| Ir al final | `End` | `G`, `End` | `End` |
| Cambiar panel | `Tab` | `Tab` | `Tab` |
| Tag / untag | `Insert`, `Space` | `v`, `Insert`, `Space` | `Space`, `Insert` |
| Execute (Enter) | `Enter` | `l`, `Enter` | `Enter` |
| Ir al padre | `Backspace` | `h`, `Backspace` | `Backspace` |

---

## 2. Modos de vista (Ctrl+1 … Ctrl+9)

Idéntico en los tres presets:

| Tecla | Modo |
| --- | --- |
| `Ctrl+1` | Brief (solo nombres) |
| `Ctrl+2` | Medium (nombre + ext) |
| `Ctrl+3` | Full (nombre, tamaño, fecha) |
| `Ctrl+4` | Wide (ancho de una columna) |
| `Ctrl+5` | Detailed (perms, owner, grupo, links) |
| `Ctrl+6` | Descriptions (`Descript.ion`) |
| `Ctrl+7` | File owners |
| `Ctrl+8` | File links |
| `Ctrl+9` | Alt Full (columnas definidas por el usuario) |

---

## 3. Toggles de panel

| Acción | Norton | Neovim | VSCode |
| --- | --- | --- | --- |
| Toggle panel izquierdo | `Ctrl+F1` | `Ctrl+F1` | `Ctrl+F1` |
| Toggle panel derecho | `Ctrl+F2` | `Ctrl+F2` | `Ctrl+F2` |
| Toggle ambos paneles | `Ctrl+O` | `Ctrl+O` | `Ctrl+B` |
| Info panel | `Ctrl+L` | `Ctrl+L` | `Ctrl+L` |
| Quick view | `Ctrl+Q` | `Ctrl+Q` | `Ctrl+Shift+Q` |
| Toggle long names | `Ctrl+N` | `Ctrl+N` | (—) |
| Swap panels | `Ctrl+U` | `Ctrl+S` | `Ctrl+Shift+S` |
| Refresh | `Ctrl+R` | `Ctrl+R` | `Ctrl+Shift+E` |
| Toggle hidden | `Ctrl+H` | `Ctrl+H` | `Ctrl+Shift+.` |
| Save setup | `Shift+F9` | `Shift+F9` | `Ctrl+S` |
| Cycle F-key modifiers | `Ctrl+P` | `Ctrl+P` | `Ctrl+P` |

---

## 4. Ordenamiento (Ctrl+F3 … Ctrl+F12)

Idéntico en los tres presets:

| Tecla | Orden |
| --- | --- |
| `Ctrl+F3` | Nombre |
| `Ctrl+F4` | Extensión |
| `Ctrl+F5` | Write time |
| `Ctrl+F6` | Tamaño |
| `Ctrl+F7` | Sin orden |
| `Ctrl+F8` | Creation time |
| `Ctrl+F9` | Access time |
| `Ctrl+F10` | Description |
| `Ctrl+F11` | Owner |
| `Ctrl+F12` | Abre el diálogo Sort Modes |

---

## 5. Acciones de archivo (F-keys + Alt/Shift)

| Acción | Norton | Neovim | VSCode |
| --- | --- | --- | --- |
| Help | `F1` | `F1` | `F1` |
| User Menu | `F2` | `F2` | `F2` |
| View | `F3` | `o`, `F3` | `Ctrl+Shift+V`, `F3` |
| Edit | `F4` | `e`, `F4` | `Ctrl+E`, `F4` |
| Copy | `F5` | `y`, `F5` | `Ctrl+C`, `F5` |
| Move / Rename+Move | `F6` | `m`, `F6` | `Ctrl+X`, `F6` |
| Rename in place | `F7` | `r`, `F7` | `F2`, `F7` |
| Make folder | (F2 → `6`) | `Ctrl+Shift+N` | `Ctrl+Shift+N` |
| Delete | `F8` | `d`, `F8` | `Delete`, `F8` |
| Quit | `F10` | `Ctrl+C`, `F10` | `Ctrl+Q`, `F10` |
| Screens list | `F12` | `F12` | `F12` |
| Next screen | `Ctrl+Tab` | `Ctrl+Tab` | `Ctrl+Tab` |
| Previous screen | `Ctrl+Shift+Tab` | `Ctrl+Shift+Tab` | `Ctrl+Shift+Tab` |
| View alternate | `Alt+F3` | `Alt+F3` | (—) |
| Print file | `Alt+F5` | `Alt+F5` | (—) |
| Create link | `Alt+F6` | `Alt+F6` | (—) |
| Secure wipe | `Alt+Delete` | `Alt+Delete` | `Shift+Delete` |
| File attributes | `Ctrl+A` | `Ctrl+A` | `Ctrl+A` |
| Apply command | `Ctrl+G` | `Ctrl+G` | (—) |
| Describe file | `Ctrl+Z` | `Ctrl+Z` | (—) |
| Compress | `Shift+F1` | `Shift+F1` | `Shift+F1` |
| Extract | `Shift+F2` | `Shift+F2` | `Shift+F2` |
| Archive commands | `Shift+F3` | `Shift+F3` | `Shift+F3` |

---

## 6. Búsqueda, historial, árbol

| Acción | Norton | Neovim | VSCode |
| --- | --- | --- | --- |
| Find file | `Alt+F7` | `/`, `Alt+F7` | `Ctrl+F` |
| Command history | `Alt+F8` | (—) | `Ctrl+Shift+H` |
| Video mode (popup de info) | `Alt+F9` | `Alt+F9` | (—) |
| Tree view | `Alt+F10` | `Alt+F10` | `Ctrl+Shift+T` |
| File view history | `Alt+F11` | `Alt+F11` | `Alt+F11` |
| Folders history | `Alt+F12` | `Alt+F12` | `Alt+F12` |
| File panel filter | `Ctrl+I` | `Ctrl+I` | `Ctrl+I` |
| Quick filter | `Ctrl+F`, `f`, `F` | `Ctrl+F`, `f`, `F` | `Ctrl+F`, `f`, `F` |
| Task list (procesos del OS) | `Ctrl+W` | `Ctrl+W` | `Ctrl+W` |
| Context menu | `Menu`, `Alt+M` | `Menu`, `Alt+M` | `Menu`, `Shift+F10` |
| SSH connect | `Ctrl+Shift+S` | `Ctrl+Shift+S` | `Ctrl+Shift+S` |
| Git panel | `Alt+G` | `Alt+G` | `Alt+G` |
| Transfer panel | `Ctrl+T` | `Ctrl+T` | `Ctrl+T` |
| Toggle reverse sort | `Ctrl+Shift+R` | (—) | (—) |
| Cycle F-key modifier | `Ctrl+P` | `Ctrl+P` | `Ctrl+P` |
| Folder shortcut 1…9 | `Ctrl+Alt+1` … `Ctrl+Alt+9` | `Ctrl+Alt+1` … `Ctrl+Alt+9` | `Ctrl+Alt+1` … `Ctrl+Alt+9` |

---

## 7. Selección en bulk

Idéntico en los tres presets:

| Acción | Tecla |
| --- | --- |
| Select group (glob) | `Gray+` (keypad) |
| Unselect group (glob) | `Gray-` (keypad) |
| Invert selection | `Gray*` (keypad) |
| Restore last selection | `Ctrl+M` |

---

## 8. Barra de menú superior (F9)

| Menú | Submenú |
| --- | --- |
| `Left` / `Right` | View mode, Info panel, Quick view, Sort modes, Sort by…, Show long names, Panel on/off, Re-read, Change drive, Connect SSH, Disconnect SSH, Git (auto-mostrado si la ruta está en un repo) |
| `Files` | View, View alt, Edit, Copy, Print, Rename/Move, Rename, Link, Make folder, Delete, Wipe, Add to archive, Extract files, Archive commands, File attributes, Apply command, Describe files, Select group, Unselect group, Invert selection, Restore selection, Plugin commands, Exit |
| `Commands` | Find file, History, Video mode, Tree view, File view hist, Folders hist, Swap panels, Panels on/off, Compare folders, User menu, Edit user menu, File associations, Folder shortcuts, File panel filter, Screens list, Task list, Hotplug devices, *(si dev mode: Install dev plugin)* |
| `Options` | Configuration…, Check for updates |
| `Help` | About…, *(este popup de ayuda es `F1`, no viene del menú)* |

---

## 9. La barra F-key (modificadores)

La barra F-key inferior tiene **cuatro vistas**. Apretá `Ctrl+P`
para ciclar.

| Slot | Default | Ctrl | Alt | Shift |
| --- | --- | --- | --- | --- |
| F1 | Help | Left | Left | — |
| F2 | User | Right | Right | — |
| F3 | View | Name | View | Compress |
| F4 | Edit | Extens | Edit | Extract |
| F5 | Copy | Time | Print | (—) |
| F6 | Move | Size | MkLink | (—) |
| F7 | Rename | Unsort | Find | (—) |
| F8 | Delete | Creatn | History | (—) |
| F9 | Menu | Access | Video | Save |
| F10 | Quit | Descr | Tree | (—) |
| F11 | (—) | Owner | ViewHist | Install dev plugin (si dev mode) |
| F12 | Screen | Sort | FoldHist | (—) |

> Los slots con `—` están sin bindear. Aún muestran el número del
> slot; el label de la F-key queda en blanco.

---

## 10. En las pantallas del editor (F4) y visor (F3)

| Acción | Tecla |
| --- | --- |
| Help | `F1` |
| Save (solo editor) | `F2` |
| Toggle text / hex | `F4` |
| Search | `F7` |
| Discard changes (solo editor) | `F8` |
| Quit | `F10` |

---

## 11. Dentro de popups

| Popup | Teclas |
| --- | --- |
| **Screens list** (`F12`) | `Up` / `Down`, `Enter`, `Esc` |
| **Help** (`F1`) | Lista: `Up` / `Down`, `Enter` para abrir, `Tab` o `Left` / `Right` para cambiar pestaña. Lector: `Up` / `Down` (o `j` / `k`), `PageUp` / `PageDown`, `Backspace` para volver a la lista, `Esc` para cerrar. |
| **File associations** | `Up` / `Down`, `A` para agregar, `E` / `Enter` para editar, `D` / `Delete` para quitar, `Esc`. |
| **Sort modes** | `Up` / `Down`, `Space` para alternar, `Enter` para aplicar, `Esc`. |
| **Configuration** | `Tab` para cambiar de pestaña, `Up` / `Down` para mover, `Space` / `Enter` para alternar / editar, `F9` para guardar, `Esc` para cancelar. |

---

## A dónde ir ahora

- Enum completo de acciones: [`43_reference_actions`](43_reference_actions.md)
- Cambiar o escribir un keymap: [`26_howto_keymaps_user_menu`](26_howto_keymaps_user_menu.md)
