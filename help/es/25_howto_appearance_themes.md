# 🔧 How-To: Apariencia, Temas y la Barra F-Key

> **Cuadrante: HOW-TO** — *orientado a problemas.*

La apariencia de Pairee se controla con tres cosas: el **tema**, las
**opciones de layout / panel**, y la **barra F-key** de abajo. Esta
página cubre cómo cambiarlas.

---

## Cambiar el tema

### Desde el diálogo de Configuración

1. Apretá `F9` → `Options` → `Configuration` (o `Commands →
   Configuration`).
2. Andá a la pestaña **Colors**.
3. La primera fila alterna el tema bundled entre **`slate`** y
   **`classic_blue`**.
4. `Enter` para aplicar; `F9` de nuevo para guardar.

### Con un archivo de tema custom

Los temas custom se cargan desde:

- **Windows:** `%APPDATA%\pairee\config\themes\`
- **Linux / macOS:** `~/.config/pairee/themes/`

Tirá un archivo `.toml` en esa carpeta, después seleccionálo desde la
pestaña **Colors**. El esquema completo está documentado en
[`42_reference_themes`](42_reference_themes.md).

---

## Editar los grupos de colores

Los grupos de colores son slots con nombre a los que podés apuntar en
tu tema custom (por ejemplo, el color de los listados de `directory`,
de `executable`, del `border` del panel activo, etc.).

1. Apretá `F9` → `Options` → `Configuration`.
2. Pestaña **Colors** → segunda fila → **Color groups**.
3. Un modal lista cada grupo. Elegí uno y `Enter`.
4. Elegí un color por nombre (`Blue`, `Yellow`, …) o hex (`#RRGGBB`).
5. `Enter` para aplicar; `Esc` para cancelar.

---

## Resaltado de archivos

La tercera fila de la pestaña **Colors** abre el editor de **Files
highlighting**. Ahí podés mapear máscaras de archivo a colores:

| Modo | Significado |
| --- | --- |
| `+H` | Archivos ocultos / de sistema |
| `+S` | Symlinks |
| `+D` | Directorios |
| `<exec>` | Ejecutables |
| `<arc>` | Archivos |
| `<temp>` | Archivos temporales |

Podés agregar reglas custom con un glob (`*.rs`) y un color.

---

## Layout y opciones de panel

Apretá `F9` → `Options` → `Configuration` → pestaña **Panel**. Las
opciones están divididas en:

- **Display & selection**: mostrar archivos ocultos, resaltar
  archivos, seleccionar carpetas, el click derecho selecciona.
- **Sorting**: ordenar por extensión, reverso, mostrar letra de orden.
- **Updates & information**: throttling de conteo de objetos,
  auto-refresh de drives de red, espacio libre, total de archivos.
- **Appearance**: títulos de columna, línea de status, scrollbar,
  conteo de pantallas de fondo, ".." en root.
- **Info panel & descriptions**: formato de hostname, opciones de
  Descript.ion.

Para la referencia completa de campos, mirá
[`41_reference_configuration`](41_reference_configuration.md), Tab 1.

---

## Opciones de interfaz

Apretá `F9` → `Options` → `Configuration` → pestaña **Interface**:

- **Clock** — muestra un reloj digital en vivo arriba a la derecha.
- **Mouse support** — alterna navegación y clicks de mouse.
- **Show bottom F-keys bar** — alterna la barra de hints F1–F12.
- **Always show the menu bar** — mantiene el menú de arriba visible.
- **Screen saver minutes** — auto-blank después de inactividad.
- **Total copy / delete progress** — muestra progreso agregado y ETA.
- **Use Ctrl+PgUp to change drive** — binding alternativo para
  cambiar de drive.
- **Use virtual terminal** — toggle de modo de consola Windows.
- **ClearType friendly redraw** — workaround para glitches de fuente
  en Windows Console.
- **Window Title Format** — tokens para la barra de título de la
  terminal.
- **Enable Yazi workflow** — `s` abre Sort, `v` abre View, solo
  cuando la línea de comandos está vacía.

Para la referencia completa de campos, mirá
[`41_reference_configuration`](41_reference_configuration.md), Tab 2.

---

## La barra F-key (modificadores)

La barra F-key inferior tiene **cuatro vistas**, cicladas con `Ctrl+P`:

| Modificador | Qué hace cada F-key |
| --- | --- |
| **Ninguno** (default) | F1 Help, F2 User, F3 View, F4 Edit, F5 Copy, F6 Move/Rename, F7 Rename, F8 Delete, F9 Menu, F10 Quit, F11 (vacía por default), F12 Screens |
| **Ctrl** | F1 Left panel, F2 Right panel, F3 Sort by Name, F4 Sort by Extension, F5 Sort by Time, F6 Sort by Size, F7 Unsorted, F8 Sort by Creation, F9 Sort by Access, F10 Sort by Description, F11 Sort by Owner, F12 Sort Modes |
| **Alt** | F1 Left drive, F2 Right drive, F3 View alt, F4 Edit alt, F5 Print, F6 Make link, F7 Find, F8 History, F9 Video, F10 Tree, F11 View history, F12 Folders history |
| **Shift** | F1 Add to archive, F2 Extract, F3 Archive commands, F9 Save setup, F11 Install dev plugin (solo dentro de un plugin dev dir) |

El ciclado es útil cuando la terminal no puede reportar el estado de
los modificadores en vivo (ej. SSH plano sin X11 forwarding). Mirá
[`30_howto_ssh_modifier_keys`](30_howto_ssh_modifier_keys.md) para la
historia completa.

---

## Tips rápidos de estilo

| Objetivo | Probá |
| --- | --- |
| Look calmo con azules sutiles | Theme = `slate`, set `panel.border = "DarkGray"`. |
| Alto contraste para un proyector | Theme = `classic_blue`, subí `panel.background` a `Black`. |
| Las carpetas resaltan | En Colors → Color groups, set `file_directory = "Yellow"`. |
| Apaciguar el badge de update ruidoso | Set `dismissed_update_version` en `settings.toml`. |
| Huella de terminal más chica | Toggle "Show bottom F-keys bar" off (`Configuration → Interface`). |

---

## A dónde ir ahora

- Esquema TOML de temas completo: [`42_reference_themes`](42_reference_themes.md)
- Cada campo de configuración: [`41_reference_configuration`](41_reference_configuration.md)
- Referencia de la barra F-key: [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
