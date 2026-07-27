# 📖 Referencia: Enum de Actions y Esquema de Keymap

> **Cuadrante: REFERENCE** — *orientado a información.*

Cada keybinding en Pairee es un mapeo de un **string de tecla** a una
**acción** (una variante del enum `Action` en
`src/keybindings/actions.rs`). Esta página documenta ambos lados así
podés leer o escribir un preset de keymap custom.

> Si solo necesitás una forma rápida de bindear unas pocas teclas,
> mirá [`26_howto_keymaps_user_menu`](26_howto_keymaps_user_menu.md)
> para el overlay de `user.toml`.

---

## 1. Formato del archivo de keymap

Un preset es un archivo TOML dentro del directorio `keymaps/`, llamado
`<preset_name>.toml`. La estructura es:

```toml
# Los comentarios empiezan con `#`

[bindings]
# action_name = "KeyCombo"
# Múltiples bindings para la misma acción están permitidos (separados por coma).
move_up   = "Up, k"
move_down = "Down, j"
quit      = "F10, Ctrl+c"
```

El stem del filename es el nombre del preset. Seleccioná el preset en
`settings.toml`:

```toml
keymap = "mi_custom"
```

> Tres presets vienen con Pairee: `norton`, `neovim`, `vscode`.

---

## 2. Sintaxis de strings de tecla

| Parte | Ejemplos | Notas |
| --- | --- | --- |
| Letra | `a`, `Z`, `0`, `_` | Un solo carácter, case exacto. |
| Especial | `Up`, `Down`, `Left`, `Right`, `Home`, `End`, `PageUp`, `PageDown`, `Backspace`, `Tab`, `Enter`, `Esc`, `Space`, `Insert`, `Delete`, `Menu` | Usá la palabra exacta. |
| Función | `F1` … `F12` | |
| Modificador | `Ctrl`, `Shift`, `Alt` | Se combinan con `+`. |
| Keypad | `Gray+`, `Gray-`, `Gray*` | `Gray` es el prefijo de keypad con `Num Lock` apagado usado en el Norton Commander original. |

Combiná modificadores con `+`:

```text
Ctrl+Shift+S
Alt+F10
Shift+Delete
Ctrl+Alt+1
```

Múltiples bindings para la misma acción son un string separado por
comas:

```toml
quit = "F10, Ctrl+c, Ctrl+q"
```

---

## 3. El enum `Action`

La lista completa, agrupada. El comentario en cada línea es el binding
default del preset Norton.

### Navegación

| Variante | Default | Efecto |
| --- | --- | --- |
| `MoveUp` | `Up` | Mueve el cursor arriba. |
| `MoveDown` | `Down` | Mueve el cursor abajo. |
| `PageUp` | `PageUp` | Una página arriba. |
| `PageDown` | `PageDown` | Una página abajo. |
| `GoToTop` | `Home` | Salta al primer item. |
| `GoToBottom` | `End` | Salta al último item. |
| `ChangePanel` | `Tab` | Cambia foco de panel. |
| `SelectItem` | `Insert`, `Space` | Taguea / destaguea. |
| `Execute` | `Enter` | Abre o corre. |
| `GoParent` | `Backspace` | Va al directorio padre. |

### Modos de vista del panel (Ctrl+1 … Ctrl+9)

| Variante | Default | Efecto |
| --- | --- | --- |
| `PanelViewBrief` | `Ctrl+1` | Solo nombres, multi-columna. |
| `PanelViewMedium` | `Ctrl+2` | Nombre + extensión. |
| `PanelViewFull` | `Ctrl+3` | Nombre, tamaño, fecha. |
| `PanelViewWide` | `Ctrl+4` | Ancho de una columna. |
| `PanelViewDetailed` | `Ctrl+5` | Perms, owner, grupo, links. |
| `PanelViewDescriptions` | `Ctrl+6` | Nombre + `Descript.ion`. |
| `PanelViewFileOwners` | `Ctrl+7` | Nombre + owners. |
| `PanelViewFileLinks` | `Ctrl+8` | Nombre + conteo de hardlinks. |
| `PanelViewAltFull` | `Ctrl+9` | Columnas definidas por el usuario. |

### Toggles de panel

| Variante | Default | Efecto |
| --- | --- | --- |
| `TogglePanelLeft` | `Ctrl+F1` | Mostrar / ocultar panel izquierdo. |
| `TogglePanelRight` | `Ctrl+F2` | Mostrar / ocultar panel derecho. |
| `ToggleBothPanels` | `Ctrl+O` | Ocultar ambos; apretá de nuevo para restaurar. |
| `InfoPanel` | `Ctrl+L` | Overlay de info. |
| `QuickView` | `Ctrl+Q` | Preview en panel pasivo. |
| `SortModes` | `Ctrl+F12` | Abre el diálogo Sort Modes. |

### Ordenamiento (Ctrl+F3 … Ctrl+F11)

| Variante | Default | Efecto |
| --- | --- | --- |
| `SortByName` | `Ctrl+F3` | Nombre. |
| `SortByExtension` | `Ctrl+F4` | Extensión. |
| `SortByWriteTime` | `Ctrl+F5` | mtime. |
| `SortBySize` | `Ctrl+F6` | Tamaño. |
| `SortUnsorted` | `Ctrl+F7` | Orden del filesystem. |
| `SortByCreationTime` | `Ctrl+F8` | Birth time. |
| `SortByAccessTime` | `Ctrl+F9` | atime. |
| `SortByDescription` | `Ctrl+F10` | `Descript.ion`. |
| `SortByOwner` | `Ctrl+F11` | Owner. |
| `ToggleSortReverse` | `Ctrl+Shift+R` | Invierte el orden actual. |

### Acciones de F-key

| Variante | Default | Efecto |
| --- | --- | --- |
| `Help` | `F1` | Abre el popup de ayuda. |
| `About` | (menú) | Diálogo About. |
| `UserMenu` | `F2` | Abre el User Menu. |
| `View` | `F3` | Ver archivo. |
| `ViewAlt` | `Alt+F3` | Ver en modo alternativo. |
| `Edit` | `F4` | Editar archivo. |
| `Copy` | `F5` | Copiar. |
| `Move` | `F6` | Mover / rename-and-move. |
| `Rename` | `F7` | Renombrar in place. |
| `MkDir` | (F2 → 6) | Crear carpeta. |
| `Delete` | `F8` | Borrar. |
| `Menu` | `F9` | Abre la barra de menú superior. |
| `Quit` | `F10` | Salir de Pairee. |
| `PluginMenu` | (menú) | Abre el Plugin Manager. |
| `ScreensList` | `F12` | Overlay de pantallas. |
| `NextScreen` | `Ctrl+Tab` | Siguiente pantalla. |
| `PrevScreen` | `Ctrl+Shift+Tab` | Pantalla anterior. |

### Operaciones de archivos

| Variante | Default | Efecto |
| --- | --- | --- |
| `PrintFile` | `Alt+F5` | Imprimir (aplica un comando de filtro custom). |
| `CreateLink` | `Alt+F6` | Link simbólico o hard. |
| `WipeFile` | `Alt+Delete` | Sobre escritura segura + borrar. |
| `FileAttributes` | `Ctrl+A` | Diálogo de atributos. |
| `ApplyCommand` | `Ctrl+G` | Corre un comando shell sobre los archivos seleccionados. |
| `DescribeFile` | `Ctrl+Z` | Edita entrada de `Descript.ion`. |
| `CompressFiles` | `Shift+F1` | Comprimir. |
| `ExtractArchive` | `Shift+F2` | Extraer. |
| `ArchiveCommands` | `Shift+F3` | Sub-menú de archivo. |

### Selección en bulk

| Variante | Default | Efecto |
| --- | --- | --- |
| `SelectGroup` | `Gray+` | Taguea por glob. |
| `UnselectGroup` | `Gray-` | Destaguea por glob. |
| `InvertSelection` | `Gray*` | Invierte. |
| `RestoreSelection` | `Ctrl+M` | Restaura el último snapshot de selección. |

### Búsqueda & historial

| Variante | Default | Efecto |
| --- | --- | --- |
| `FindFile` | `Alt+F7` | Buscar archivos (nombre o contenido). |
| `CommandHistory` | `Alt+F8` | Historial de línea de comandos. |
| `FileViewHistory` | `Alt+F11` | Archivos abiertos con `F3` / `F4`. |
| `FoldersHistory` | `Alt+F12` | Carpetas visitadas recientemente. |

### Comandos

| Variante | Default | Efecto |
| --- | --- | --- |
| `CompareFolder` | (menú) | Compara los dos paneles. |
| `EditUserMenu` | (menú) | Abre el editor del User Menu. |
| `FileAssociations` | (menú) | Abre el editor de File Associations. |
| `FolderShortcutsConfig` | (menú) | Abre el diálogo de Folder Shortcuts. |
| `FilePanelFilter` | `Ctrl+I` | Setea un filtro persistente en el panel activo. |
| `QuickFilter` | `Ctrl+F` (o `f` / `F`) | Filtro de substring en vivo. |
| `TaskList` | `Ctrl+W` | Lista de procesos del OS. |

### Opciones

| Variante | Default | Efecto |
| --- | --- | --- |
| `SaveSetup` | `Shift+F9` | Persiste todos los settings inmediatamente. |
| `SystemSettings` | (menú) | Abre el diálogo de Configuration. |
| `CheckForUpdates` | (menú) | Abre el diálogo de update. |

### General

| Variante | Default | Efecto |
| --- | --- | --- |
| `ToggleHidden` | `Ctrl+H` | Mostrar / ocultar archivos ocultos. |
| `FocusCli` | (N/A) | Enfoca la línea de comandos. |
| `Unfocus` | `Esc` | Desenfoca / cierra popup. |
| `Refresh` | `Ctrl+R` | Refresca ambos paneles. |
| `SwapPanels` | `Ctrl+U` | Swappea izquierdo y derecho. |
| `DriveSelectLeft` | `Alt+F1` | Menú de drives (izq). |
| `DriveSelectRight` | `Alt+F2` | Menú de drives (der). |
| `ContextMenu` | `Menu`, `Alt+M` | Menú contextual. |
| `GoFolderShortcut(u8)` | `Ctrl+Alt+1` … `Ctrl+Alt+9` | Salta a un atajo de carpeta guardado. |
| `ToggleLongNames` | `Ctrl+N` | Alterna el rendering de nombres largos. |
| `RereadPanel` | `Ctrl+R` | Re-lee el panel activo. |
| `VideoMode` | `Alt+F9` | Muestra un diálogo de hint de video-mode. |
| `TreeView` | `Alt+F10` | Overlay de árbol. |
| `CycleFKeysModifiers` | `Ctrl+P` | Cicla la barra F-key (Normal/Ctrl/Alt). |
| `SshConnect` | `Ctrl+Shift+S` | Diálogo de conexión SSH. |
| `SshDisconnect` | (menú) | Desconecta el panel activo. |
| `OpenGitPanel` | `Alt+G` | Abre el dashboard Git. |
| `InstallDevPlugin` | (dev mode) | Instala un plugin dev local. |
| `ToggleTransferPanel` | `Ctrl+T` | Mostrar / ocultar el Transfer Panel. |

---

## 4. Wildcards / múltiples teclas

La misma `Action` se puede bindear a múltiples teclas. Pairee dispara
la acción en la **primera tecla que matchea**. Esta es también la
forma en que la barra F-key muestra el binding actualmente en efecto.

```toml
[bindings]
quit = "F10, Ctrl+c, Ctrl+q"
```

---

## 5. Desbindeo de una tecla

No hay sintaxis explícita de "desbindear". Para deshabilitar un
binding, copiá el preset a un archivo custom y **sacá** la línea.

---

## 6. El overlay `user.toml`

Si solo querés sobreescribir unas pocas teclas sin copiar un preset
completo, tirá tus cambios en `keymaps/user.toml`. Las entradas en
`user.toml` se mergean encima del preset activo; el preset activo gana
para las teclas que no sobreescribís.

---

## A dónde ir ahora

- Los tres presets lado a lado: [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
- Cambiar de preset: [`26_howto_keymaps_user_menu`](26_howto_keymaps_user_menu.md)
