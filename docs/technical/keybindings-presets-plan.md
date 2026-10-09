# Pairee — Plan: presets de atajos, modal de atajos y atajos de plugins

> **Fecha:** 2026-10-09
> **Rama de trabajo:** `master`
> **Objetivo:** que quien venga de un programa estándar (Explorer / VS Code / Total Commander), de Norton Commander / Far Manager, de Neovim o de yazi use Pairee **sin reaprender atajos**. Además: un modal que liste todos los atajos activos (núcleo y plugins), editable desde la app, y un modelo de atajos para plugins integrado en el mismo resolver.

Cada ítem usa checkbox (`[ ]` pendiente / `[x]` hecho, con commit). Actualizar entre tarea y tarea.

---

## 1. Diagnóstico del sistema actual

### 1.1 Qué hay hoy

| Pieza | Ubicación | Estado |
|---|---|---|
| Enum `Action` (121 variantes, 2 con parámetro) | `src/keybindings/actions.rs` | Completo para panel; no cubre editor ni visor. |
| Tabla nombre → acción + sufijos alias (`_arrow`, `_fkey`, …) | `src/keybindings/preset.rs:22-170` | Los sufijos sobran: el formato ya admite varios chords (`"a, b"`). |
| Catálogo (`CATALOG`, ~58 entradas, `ActionCategory`) | `src/keybindings/registry.rs` | Incompleto (faltan sort, views, menús…). `ActionCategory` no se usa en ningún lado. Las etiquetas no están localizadas: el id con `_` → espacio. |
| Carga y validación (crate `keybinds` 0.2) | `src/keybindings/loader/*` | Soporta secuencias (`"g g"`), pero ningún preset las usa. |
| Resolver | `src/keybindings/resolver/mod.rs` | Una sola capa global (panel). `key_for_action` devuelve un solo chord por acción. No hay timeout de secuencia. |
| Presets `norton`, `neovim`, `vscode` | `keymaps/*.toml` | ~70 % del contenido está copiado entre los tres (sort, views, folder shortcuts, tabs, F-keys). |
| Which-key (overlay filtrable) + HUD de prefijo | `src/app/actions/which_key.rs`, `src/ui/which_key_prefix.rs` | Lee el keymap vivo. Sin categorías, sin scroll real (`FilterListView`) y sin edición. |
| Paleta de comandos | `src/app/actions/command_palette.rs` | No muestra chords. El título fija "Ctrl+Shift+P" aunque el preset diga otra cosa. |
| Ayuda F1 `help/*/keyboard_shortcuts.md` | estático | Se escribe a mano, así que se desincroniza de los presets. |
| Barra de F-keys y hints de menú | `src/ui/fkeys.rs`, `src/ui/menu/types.rs` | Leen el keymap vivo (bien), salvo las filas de editor y visor, que están fijas. |
| Atajos de plugins | `src/plugin/registry.rs:40,69-75,249` y `events.rs:88-93` | Van en un mapa aparte que solo se consulta como último recurso (detalle en §1.3). |

### 1.2 Bugs y deudas que bloquean el objetivo

1. **`custom_bindings` no puede sobrescribir.** En el loader gana el primer chord registrado (`loader/mod.rs:112`). Si el usuario reasigna un chord que el preset ya usa, sale un error "Duplicate" y no se aplica. Además, una acción personalizada *añade* el chord nuevo pero no quita el del preset. Sin esto no hay personalización real.
2. **Orden no determinista.** El preset y los overrides son `HashMap`, así que cuando dos filas chocan el ganador depende del hash.
3. **El nombre del preset se guarda en dos campos:** `settings.keybinding_preset` y `keybindings.preset`. El test `every_keymap_binds_undo_and_redo` cambia el campo equivocado y en realidad prueba `norton` tres veces.
4. **`cli.rs:25` compara `preset == "vim"`**, pero el preset real se llama `"neovim"`, así que esa rama nunca se ejecuta. En neovim, cualquier letra sin asignar empieza a escribir en la CLI.
5. **Yazi `s`/`v` cableado a mano** en `events.rs:49-61`, antes del keymap. Le roba la `v` al preset neovim.
6. **El editor y el visor dejan pasar solo F12 y Ctrl+Tab, fijos en el código.** Si el usuario reasigna esas acciones, el editor y el visor no se enteran. Ctrl+Shift+Tab llega como `BackTab` y nunca coincide.
7. **El sembrado de presets pisa los archivos del usuario** cuando `has_outdated_layout` detecta el layout viejo (`config/loading.rs:141`).
8. **`include_str!` de los presets duplicado** en `preset.rs:8-10` y `loader/disk.rs:5-7`.
9. **Semántica dudosa en los presets actuales:**
   - neovim: `go_to_top = "g"` debería ser `g g`. `/` abre la búsqueda recursiva, cuando en vim es búsqueda en la vista. `d` borra al instante (vim pide `dd`). `Ctrl+c` sale de la app. `Space` selecciona, aunque es el leader habitual.
   - Los tres presets ponen `f`/`F` como filtro rápido: en norton eso impide escribir "f" en la línea de comandos vacía.
   - `Ctrl+i` = Tab, `Ctrl+m` = Enter y `Ctrl+h` = Backspace en terminales legacy. Norton y vscode los usan y chocan con `change_panel`, `execute` y `go_parent`.
   - `Ctrl+Shift+<letra>` solo se distingue de `Ctrl+<letra>` con el protocolo de teclado kitty. Hoy `Ctrl+p` (cycle F-keys) y `Ctrl+P` (paleta) colisionan en conhost/xterm.
   - No existe un portapapeles de archivos (yank/cut/paste). Es imprescindible para el preset estándar (`Ctrl+C/X/V`), neovim (`yy`/`p`) y yazi (`y`/`x`/`p`). Hoy `vscode` mapea `Ctrl+c` a "copiar al otro panel", que es otra semántica.

### 1.3 Plugins: estado

- El manifiesto declara `[keybindings] "tecla" = "accion"`. Esos atajos van a un mapa global aparte que **solo se consulta si el núcleo no resolvió la tecla**:
  - el núcleo siempre gana, en silencio;
  - entre plugins gana el último que escribe, en orden no determinista porque los plugins cargan en paralelo;
  - no se admiten secuencias.
- **El formato de tecla no coincide.**
  - La documentación y la plantilla usan `"ctrl+h"` y el test usa `"ctrl-p"`. La búsqueda compara exacto contra `"Ctrl+h"`, así que los atajos con modificador nunca disparan.
  - Ya existe un normalizador (`plugin/manager/dialogs.rs:85-115`), pero solo lo usa `pairee.which`.
- Nunca se desregistran atajos al desinstalar, aunque la guía dice que sí.
- **No aparecen en ninguna superficie:** ni en la paleta, ni en which-key, ni en la ayuda. Tampoco existe `plugin:<name>`, que la guía documenta.
- `entry(args)` recibe el nombre de la acción en `args[1]`. La documentación dice "llama al método `run_action()`", y eso es falso.
- El packager copia `keybindings` al campo `hooks` del registro (`registry_pack.rs:168-171`).
- `pairee.emit` solo soporta `cd` y `focus`.

---

## 2. Principios de diseño

1. **Un solo resolver, una sola fuente de verdad.** Núcleo, plugins, which-key, paleta, barra de F-keys, menús, ayuda y el nuevo modal leen del mismo `Keymap`. No hay mapas paralelos.
2. **Capas con herencia, sin copiar y pegar.**
   - Orden: `base` → preset → defaults de plugins → overrides del usuario.
   - Gana la última capa. Cuando una capa toma un chord ocupado, desplaza a la acción anterior y el reporte lo registra como *aviso*, no como error.
   - Dentro de un mismo archivo, un duplicado sigue siendo error.
3. **Contextos explícitos.** `panels`, `editor`, `viewer`, `list` (navegación genérica de popups), `prompt` (campos de texto). Un mismo chord puede significar cosas distintas en contextos distintos sin que sea conflicto.
4. **Robustez en terminal.** Toda acción esencial tiene al menos un chord "robusto" en cada preset, es decir, uno que funciona sin protocolo kitty. Los chords frágiles se marcan y el modal los atenúa cuando el terminal no los soporta.
5. **Fidelidad con criterio.**
   - Se copia el atajo del programa de origen cuando la semántica es equivalente.
   - Cuando Pairee tiene algo que el origen no tiene (dual panel, Git, SSH, pestañas), se elige el chord que *ese* usuario adivinaría por analogía.
   - Esas decisiones se documentan como "extensión" en el propio TOML.
6. **El modo de tecleo es parte del preset.** Qué hacen las letras sin modificador depende de la cultura del programa de origen:
   - `cli` en Far/NC: las letras van a la línea de comandos;
   - `type_ahead` en Explorer: saltan al archivo que empieza así;
   - `commands` en vim/yazi: las letras son comandos.

---

## 3. Arquitectura objetivo

### 3.1 Formato de preset v2

```toml
# keymaps/neovim.toml
extends = "norton"                    # hereda F-keys y Ctrl+F* como fallbacks robustos

[options]
typing           = "commands"         # cli | type_ahead | commands
leader           = "Space"            # se expande en "<leader>"
sequence_timeout = 1000               # ms; 0 = esperar indefinidamente (yazi)

[panels]
move_down   = "j, Down"
go_to_top   = "g g, Home"
delete      = "d d"
select_item = "t, Insert"
unselect_group = ""                   # "" = desasignar lo heredado
help_keys   = "g ?, <leader>?"

[editor]   # fase 3
[viewer]   # fase 3
[list]     # fase 3
```

- Se acepta `[bindings]` como alias de `[panels]` por compatibilidad.
- Los sufijos alias (`_arrow`, `_fkey`…) quedan *deprecated*: se leen con un aviso y se eliminan de los presets incluidos.
- `extends` admite cadena, y el loader detecta ciclos.
- Archivo nuevo `keymaps/base.toml` (oculto en el selector) con lo que es universal y no choca con nada:
  - `Ctrl+Alt+1..9` → atajos de carpetas;
  - `Menu` y `Shift+F10` → menú contextual;
  - `Ctrl+Shift+P` → paleta, más un fallback robusto por preset;
  - `Ctrl+F3..F12` → ordenación;
  - pestañas con `Alt+PgUp/PgDn`.

### 3.2 Modelo de datos

```text
KeyContext { Panels, Editor, Viewer, List, Prompt }

Command = Core(Action) | Editor(EditorAction) | Viewer(ViewerAction)
        | List(ListAction) | Plugin(PluginCommandId)    // PluginCommandId = Arc<str> "plugin.<name>.<cmd>"

trait Bindable {                     // un único trait, lo implementan todos los enums de acción
    fn id(&self) -> Cow<str>;        // snake_case, estable, usado en TOML y config
    fn label_key(&self) -> &str;     // clave i18n "action_<id>"
    fn category(&self) -> Category;  // Navigation, Selection, Files, Clipboard, Search, View,
                                     // Sort, Tabs, Panels, Git, Remote, Tools, System, Plugin
    fn context(&self) -> KeyContext;
    fn essential(&self) -> bool;     // exige chord robusto en cada preset
}

Keymap {
    layers: EnumMap<KeyContext, Keybinds<Command>>,
    origin: HashMap<(KeyContext, KeySeq), BindingOrigin>,   // Base | Preset(name) | Plugin(name) | User
    report: KeymapLoadReport,                                // errores, avisos, desplazamientos, frágiles
}
```

- `CATALOG` se convierte en la implementación de `Bindable` y pasa a ser completo. Un test exige que **toda** variante tenga entrada, con `label_key` existente en `lang/en.toml` y `lang/es.toml`.
- `key_for_action` devuelve `&[KeySeq]`, todos los chords. Así la UI elige el primero robusto.
- Se reemplaza `ACTION_NAMES` + sufijos por `Bindable::id`, con un único `from_id` generado a partir del catálogo.

### 3.3 Overrides del usuario

```toml
# config.toml
[keybindings]
preset = "neovim"

[keybindings.overrides.all]          # se aplica sobre cualquier preset
open_git_panel = "Alt+g"

[keybindings.overrides.neovim]       # solo con este preset
"editor.save" = "<leader>w"
"plugin.git-blame.toggle" = "<leader>gb"
delete = ""                          # desasignar
```

- Se elimina `settings.keybinding_preset` (bug 3), con migración que lee el valor viejo una vez.
- `custom_bindings` se migra a `overrides.all`.

### 3.4 Despacho de entrada

`handle_input_event` queda como **pipeline de etapas** (Chain of Responsibility):

```text
Popup  →  Screen(editor|viewer)  →  Panels
             │                         ├─ resolver(Panels)       ← antes de la CLI si la CLI está vacía
             │                         ├─ typing mode: cli | type_ahead | ignore
             │                         └─ on_key hook (observador)
             └─ resolver(Editor|Viewer); si no hay match → manejo propio (inserción de texto)
```

- Desaparecen el `preset == "vim"` y el cableado de yazi `s`/`v`: `s`/`v` pasan a ser bindings del preset yazi.
- Desaparece el pass-through fijo de F12/Ctrl+Tab. En editor y visor, si un chord no está en su capa, se consulta la capa `Panels` solo para acciones marcadas `global` (screens, quit, paleta, modal de atajos).
- El timeout de secuencia vive en el resolver. Al vencer, el prefijo pendiente se descarta.
  - Excepción: si el prefijo pendiente es en sí un chord completo, se ejecuta. Esto solo pasa en capas de usuario, porque la validación lo prohíbe en los presets.

### 3.5 Robustez de teclado

- Tabla `fragility(KeySeq) -> Option<Reason>` con las causas conocidas:
  - `Ctrl+Shift+<letra>`, `Ctrl+Enter`, `Ctrl+<dígito>` → necesitan protocolo kitty;
  - `Ctrl+i` = Tab, `Ctrl+m` = Enter, `Ctrl+h` = Backspace, `Ctrl+[` = Esc;
  - `Alt+F4` / `F11` / `F10` capturados por el gestor de ventanas o el terminal;
  - `Ctrl+s` / `Ctrl+q` como XON/XOFF en ttys sin raw.
- Se activa `PushKeyboardEnhancementFlags` (crossterm) cuando `supports_keyboard_enhancement()` lo permite.
- El loader emite un aviso para cada acción `essential` sin chord robusto.
- El modal muestra los chords frágiles con `≈` y atenuados si el protocolo no está activo.

---

## 4. Acciones nuevas necesarias

Sin estas acciones, los presets no pueden ser fieles.

| Acción | Para quién | Notas |
|---|---|---|
| `yank`, `cut`, `paste`, `paste_overwrite`, `paste_as_link`, `clear_clipboard` | estándar, neovim, yazi | Portapapeles de archivos interno. `paste` pega en el cwd del panel activo y reutiliza el motor de transferencia y el undo. El portapapeles se ve en la barra de estado. |
| `trash` / `delete_permanent` | todos | Separar papelera de borrado definitivo (Delete / Shift+Delete; `dd`/`DD`; `d`/`D`). Verificar el soporte de papelera en `fs::journal`. |
| `select_all`, `unselect_all`, `visual_mode`, `toggle_select_and_down` | todos | `Insert`/`Space` en NC avanzan el cursor; `v`/`V` en vim y yazi. |
| `history_back`, `history_forward` | estándar, neovim, yazi | `Alt+←/→`, `Ctrl+o`/`Ctrl+i`, `H`/`L`. |
| `go_home`, `go_root` | todos | `g h`, `Ctrl+\` (Far: raíz). |
| `half_page_up/down` | neovim, yazi | Separar de `page_up/down`. |
| `find_in_panel`, `find_next`, `find_prev` | todos | Búsqueda incremental en la vista (`/`, `n`, `N`; en NC, quick search `Alt+letra`). Distinta de `find_file`, que es recursiva. |
| `search_content` | estándar, yazi | `Ctrl+Shift+F`, `S` (rg). |
| `focus_left_panel`, `focus_right_panel` | neovim | `Ctrl+w h` / `Ctrl+w l`. |
| `copy_name`, `copy_name_no_ext`, `copy_dir_path` | yazi, norton, neovim | `c f`/`c n`/`c d`, `Ctrl+Insert`. |
| `new_file`, `create` (archivo o carpeta si termina en `/`) | estándar, neovim, yazi, norton (`Shift+F4`) | |
| `rename_basename` | neovim, yazi | `c W`; el cursor queda antes de la extensión. |
| `cycle_panel_view`, `cycle_sort` | estándar | Fallback robusto para presets sin `Ctrl+1..9`. |
| `keyboard_shortcuts` | todos | Abre el modal (§7). Sustituye a `WhichKey`; se mantiene el alias `which_key`. |
| `quit_without_saving_state` | neovim, yazi | `Z Q`, `Q`. |

---

## 5. Los cuatro presets

Se renombra `vscode` → **`standard`**; `vscode` y `modern` quedan como alias. Los nombres finales son `standard`, `norton`, `neovim` y `yazi`.

Herencia:

```text
base ─┬─ standard
      └─ norton ─┬─ neovim   (hereda F-keys y Ctrl+F* como red de seguridad)
                 └─ yazi
```

**Leyenda:** `a b` = secuencia; `≈` = chord frágil (lleva fallback); *ext.* = extensión de Pairee sin equivalente en el origen, elegida por analogía.

### 5.1 Navegación

| Acción | Standard | Norton/Far | Neovim | Yazi | Criterio |
|---|---|---|---|---|---|
| Arriba / abajo | `↑`/`↓` | `↑`/`↓` | `k`/`j`, `↑`/`↓` | `k`/`j`, `↑`/`↓` | |
| Página | `PgUp`/`PgDn` | `PgUp`/`PgDn` | `Ctrl+b`/`Ctrl+f`, `PgUp`/`PgDn` | `Ctrl+b`/`Ctrl+f` | vim: pantalla completa |
| Media página | — | — | `Ctrl+u`/`Ctrl+d` | `Ctrl+u`/`Ctrl+d` | |
| Inicio / fin | `Home`/`End` | `Home`/`End` | `g g`/`G` | `g g`/`G` | Se corrige el `g` suelto. |
| Abrir / entrar | `Enter` | `Enter`, `Ctrl+PgDn` | `l`, `Enter` | `l`, `Enter`, `o` | |
| Padre | `Backspace`, `Alt+↑` | `Backspace`, `Ctrl+PgUp` | `h`, `-` | `h` | `-` como en vim-vinegar/oil.nvim. |
| Atrás / adelante | `Alt+←`/`Alt+→` | — (`Alt+F12` historial) | `Ctrl+o` / `Ctrl+i`≈ | `H`/`L` | `Ctrl+i` = Tab en terminales legacy; con kitty funciona. |
| Home / raíz | — / — | — / `Ctrl+\` | `g h` / `g /` | `g h` / `g /` | Far: `Ctrl+\` = raíz (hoy está mal asignado a hotlist). |
| Cambiar panel | `Tab` | `Tab` | `Tab`, `Ctrl+w w` | `Tab` | yazi usa `Tab` para "spot"; en un dual panel `Tab` es inevitable (desviación documentada). |
| Panel izq. / der. | — | — | `Ctrl+w h` / `Ctrl+w l` | — | |
| Intercambiar paneles | `Ctrl+U` (TC) | `Ctrl+U` | `Ctrl+w x` | paleta | |
| Ocultar paneles | `Ctrl+B` (VS Code sidebar) | `Ctrl+O` | `Ctrl+w o` | — | |

### 5.2 Selección

| Acción | Standard | Norton/Far | Neovim | Yazi | Criterio |
|---|---|---|---|---|---|
| Marcar | `Space`, `Ctrl+Space`, `Shift+↑/↓` | `Insert`, `Shift+↑/↓`, `Space` (CLI vacía) | `t` (vifm), `Insert` | `Space` (marca y baja) | En neovim, `Space` es leader. |
| Modo visual | — | — | `v` / `V` | `v` / `V` | |
| Todo / nada | `Ctrl+A` / `Esc` | `Shift+Plus` / `Shift+-` (Far) | `<leader>a` / `<leader>A` | `Ctrl+a` / `Esc` | `Ctrl+A` en Far = atributos, se mantiene. |
| Invertir | `Ctrl+Alt+I`, `*` | `*` (gray) | `<leader>i` | `Ctrl+r` | |
| Por patrón | `+` / `-` | `+` / `-` (gray) | `<leader>+` / `<leader>-` | `+` / paleta (`-` es symlink en yazi) | |

### 5.3 Operaciones de archivo

| Acción | Standard | Norton/Far | Neovim | Yazi | Criterio |
|---|---|---|---|---|---|
| Copiar al otro panel | `F5` | `F5` | `F5` (heredado) | `F5` (heredado) | Dual panel: convención TC/NC. |
| Mover al otro panel | `F6` | `F6` | `F6` | `F6` | |
| Yank / cut / paste | `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | — (no existe en NC) | `y y` / — / `p` (copia), `P` (mueve) | `y` / `x` / `p`, `P` (sobrescribe) | vifm: `P` = put con move. |
| Limpiar portapapeles | `Esc` | — | `<leader>y` | `Y` / `X` | |
| Enlace | `Ctrl+Alt+L` (ext.) | `Alt+F6` | `<leader>l` | `-` (abs.) / `_` (rel.) | |
| Papelera | `Delete` | `F8` | `d d` | `d` | vifm `dd` = papelera. |
| Borrado definitivo | `Shift+Delete` | `Shift+Delete` | `D D` | `D` | |
| Wipe | paleta | `Alt+Delete` | paleta | paleta | |
| Renombrar | `F2` | `Shift+F6` | `c w` / `c W` (basename) | `r` | D1: se vuelve al original de NC/Far. |
| Renombrado múltiple | `Ctrl+Shift+M`≈, paleta | `Ctrl+Shift+F6`≈, paleta | `<leader>r` | `R` (ext.) | `Ctrl+M` = Enter en legacy. |
| Nueva carpeta | `Ctrl+Shift+N`≈, `F7` | `F7` | `a` (con `/` final) | `a` (con `/` final) | Explorer y TC; nvim-tree y yazi. |
| Nuevo archivo | `Ctrl+N` (VS Code) | `Shift+F4` | `a` | `a` | |
| Ver | `F3`, `Alt+P` (vista previa) | `F3`, `Ctrl+Q` (quick view) | `o`, `F3` | `Enter` en archivo, `o` | |
| Editar (editor interno) | `F4`, `Ctrl+E` | `F4` | `e`, `F4` | `e` (ext.), `F4` | Siempre el editor interno, nunca uno externo. |
| Abrir con… | `Shift+F10` / `Menu` | `Menu` | `g x` (abre con) | `O` | `g x` es el open de vim. |
| Copiar ruta | `Ctrl+Shift+C`≈, `Alt+Shift+C` | `Alt+Shift+Insert` | `y p` | `c c` | |
| Copiar nombre | — | `Ctrl+Insert` | `y n` | `c f` (`c n` sin extensión) | |
| Propiedades / atributos | `Alt+Enter` | `Ctrl+A` | `<leader>.` | — | |
| Undo / redo | `Ctrl+Z` / `Ctrl+Y`, `Ctrl+Shift+Z` | `Alt+Backspace` / `Alt+Shift+Backspace` | `u` / `Ctrl+r` | `u` / `U` (ext.) | Far no tiene undo. |
| Comprimir / extraer | paleta, menú | `Shift+F1` / `Shift+F2` | `<leader>z` / `<leader>x` | paleta | |

### 5.4 Búsqueda, vista y orden

| Acción | Standard | Norton/Far | Neovim | Yazi |
|---|---|---|---|---|
| Buscar en la vista | escribir (type-ahead), `Ctrl+F` | `Alt+letra` (quick search NC) | `/`, `n`, `N` | `/`, `?`, `n`, `N` |
| Filtrar | `Ctrl+F` | `Ctrl+I`≈, `Ctrl+Alt+I` | `<leader>/` | `f` |
| Buscar archivos (recursivo) | `Ctrl+Alt+F` | `Alt+F7` | `<leader>ff` | `s` (fd), `z` (fzf) |
| Buscar contenido | `Ctrl+Shift+F`≈ (VS Code), `Ctrl+Alt+F` con texto | `Alt+F7` (con texto) | `<leader>sg` | `S` (rg) |
| Historial de carpetas | `Alt+↓` | `Alt+F12` | `<leader>fr` | `Z` (zoxide) |
| Ocultos | `Ctrl+H`≈, `Ctrl+.` | `Ctrl+H` | `g .` (oil), `z a` (vifm) | `.` |
| Refrescar | `Ctrl+R` (auto-refresh hace innecesario `F5`) | `Ctrl+R` | `Ctrl+l` (redraw) | — (auto) |
| Modos de vista | `Ctrl+Shift+1..8`≈ (Explorer), `cycle_panel_view` `Ctrl+Alt+V` | `Ctrl+1..9` | `<leader>v1..9` | `m s`, `m m`, `m p`, `m o`, `m n` (linemode → vista equivalente) |
| Info / quick view | `Alt+Enter` / `Alt+P` | `Ctrl+L` / `Ctrl+Q` | `K` (hover) / `g p` | `i` (ext.) / preview siempre visible |
| Ordenar | menú, `cycle_sort` `Ctrl+Alt+S` | `Ctrl+F3..F12` | `g s n/e/s/m/c` | `, a/n/e/s/m/b` (+ mayúscula = invertir) |

### 5.5 Pestañas, pantallas y aplicación

| Acción | Standard | Norton/Far | Neovim | Yazi |
|---|---|---|---|---|
| Nueva / cerrar pestaña | `Ctrl+T` / `Ctrl+W` | `Ctrl+Alt+T` / `Ctrl+Alt+W` | `<leader>tn` / `<leader>tc` | `t` / `Ctrl+c` |
| Siguiente / anterior | `Ctrl+Tab`≈, `Ctrl+PgDn` / `Ctrl+Shift+Tab`≈, `Ctrl+PgUp` | `Alt+PgDn` / `Alt+PgUp` | `g t` / `g T` | `]` / `[` |
| Mover pestaña | `Ctrl+Shift+PgUp/PgDn` | `Alt+Shift+PgUp/PgDn` | `<leader>t<` / `<leader>t>` | `{` / `}` |
| Ir a pestaña N | `Alt+1..9` (`Ctrl+1..9`≈) | `Alt+1..9` | `Alt+1..9` | `1..9` |
| Pantallas (editor/visor) | `F12` / `Ctrl+F12` | `F12`, `Ctrl+Tab` | `F12` | `F12` |
| Paleta de comandos | `Ctrl+Shift+P`≈, `Ctrl+Alt+P` | `Ctrl+Shift+P`≈, `Ctrl+Alt+P` | `:` | `Ctrl+Shift+P`≈, `Alt+x` (`:` es shell en yazi) |
| Línea de comandos / shell | ``Ctrl+` `` | escribir directamente | `!` | `;`, `:` |
| Menú principal | `F10`, `Alt` | `F9` | `F9` | `F9` |
| Menú de usuario | paleta | `F2` | `<leader>u` | paleta |
| Ayuda | `F1` | `F1` | `F1` | `F1` |
| **Modal de atajos** | `Ctrl+K Ctrl+S` (VS Code), `Ctrl+/` | `Ctrl+K`, entrada en `F1` | `g ?` (nvim-tree), `<leader>?` | `~`, `F1` |
| Ajustes | `Ctrl+,` | `Alt+Shift+F9`, `F9`→Opciones | `<leader>,` | paleta |
| Git | `Ctrl+Shift+G`≈, `Alt+G` | `Ctrl+Alt+G` | `<leader>gg` | `Alt+g` (`g g` es ir al inicio) |
| SSH | `Ctrl+Alt+S` | `Ctrl+Alt+R` (remoto), menú | `<leader>ss` | paleta |
| Tareas / transferencias | `Ctrl+Alt+T` | `Ctrl+W` / `Ctrl+T` | `<leader>w` | `w` |
| Salir | `Ctrl+Q` | `F10` | `Z Z`, `Z Q` (sin estado), `<leader>qq` | `q`, `Q` |

### 5.6 Modo de tecleo por preset

| Preset | `typing` | Efecto |
|---|---|---|
| standard | `type_ahead` | Las letras saltan al archivo que empieza así (buffer de 1 s, como Explorer/TC). `Ctrl+`` ` o `:` → CLI. |
| norton | `cli` | Comportamiento Far: las letras van a la CLI. `Alt+letra` = quick search. |
| neovim | `commands` | Las letras sin binding se ignoran y el HUD muestra "no mapeado". La CLI se abre con `!`. |
| yazi | `commands` | Igual que neovim; la CLI se abre con `;` / `:`. `sequence_timeout = 0` (yazi espera). |

### 5.7 Contextos editor, visor y listas (fase 3)

- **Editor.** standard = CUA, como hoy (`Ctrl+S`, `Ctrl+F`, `Ctrl+Z`…); norton = Far editor (`F2` guardar, `F7` buscar, `Shift+F7` siguiente, `F10` salir). Hoy está todo fijo en el código.
  - Para neovim **no** se implementa un modo modal completo en este plan: queda como fase futura (editor modal). Se mapean solo `Esc`→salir, `Ctrl+s`/`:w`, `/`, `n`/`N`, `u`/`Ctrl+r`.
- **Visor.** standard/norton = actual; neovim/yazi = `j`/`k`/`g g`/`G`/`Ctrl+d`/`Ctrl+u`, `/`, `n`/`N`, `q` sale.
- **Listas y popups.** `list_nav.rs` ya tiene `ListKeys::ARROWS_VIM`. La capa `list` decide por preset si `j`/`k` navegan (neovim/yazi) o filtran (standard/norton). Así se elimina la elección fija por popup.

### 5.8 Norton: atajos de NC intocables y reubicación de extensiones (D7)

**Regla:** todo atajo que exista en NC/Far conserva su significado. Las extensiones de Pairee que chocaban se mueven a chords libres. Las búsquedas rápidas y la línea de comandos funcionan como en Far.

| Chord NC/Far | Significado NC/Far (se respeta) | Antes en Pairee | Nuevo atajo de la acción desplazada |
|---|---|---|---|
| `Alt+<letra>` | Quick search en el panel | `Alt+T` nueva pestaña, `Alt+W` cerrar pestaña, `Alt+G` Git, `Alt+S` tamaños de carpeta, `Alt+D` uso de disco, `Alt+M` menú contextual, `Alt+C`/`Alt+E` comprimir/extraer, `Alt+O` abrir en pestaña nueva | `Ctrl+Alt+T`, `Ctrl+Alt+W`, `Ctrl+Alt+G`, `Ctrl+Alt+S` (y `F3` sobre carpeta, como Far), `Ctrl+Alt+D`, `Ctrl+Alt+M` (y la tecla `Menu`), solo `Shift+F1`/`Shift+F2` (eran alias), `Ctrl+Alt+O` |
| `F7` | Crear carpeta | renombrar | renombrar → `Shift+F6` (D1); renombrado múltiple → `Ctrl+Shift+F6`≈ + paleta |
| `Ctrl+\` | Ir a la raíz | hotlist | hotlist → `Ctrl+Alt+H` |
| `Ctrl+Enter` | Insertar el nombre del archivo en la CLI | abrir en pestaña nueva | acción nueva `insert_name_to_cli`; abrir en pestaña → `Ctrl+Alt+O` |
| `Ctrl+F` | Insertar la ruta completa en la CLI | filtro rápido (`Ctrl+f`, `f`, `F`) | acción nueva `insert_path_to_cli`; filtro rápido → `Ctrl+Alt+F`; se quitan `f`/`F` (las letras van a la CLI) |
| `Ctrl+[` / `Ctrl+]` | Insertar la ruta del panel izquierdo / derecho en la CLI | — | acciones nuevas `insert_left_path_to_cli` / `insert_right_path_to_cli`. `Ctrl+[` = `Esc` en terminales legacy (≈) |
| `Ctrl+P` | Ocultar/mostrar el panel inactivo | ciclar modificadores de la barra de F-keys | acción nueva `toggle_inactive_panel`; ciclar F-keys → `Ctrl+Alt+K` |
| `Ctrl+B` | Ocultar/mostrar la barra de F-keys | — | acción nueva `toggle_keybar` |
| `Ctrl+Y` | Borrar la línea de comandos | rehacer | rehacer → `Alt+Shift+Backspace` |
| `Ctrl+E` / `Ctrl+X` | Comando anterior / siguiente del historial | — | acciones nuevas `cli_history_prev` / `cli_history_next` |
| `Ctrl+Shift+S` (≈ `Ctrl+S`) | `Ctrl+S` = cursor a la izquierda en la CLI (WordStar) | conectar SSH | SSH → `Ctrl+Alt+R` |
| `Ctrl+Shift+P` (≈ `Ctrl+P`) | — (choca con `Ctrl+P` en terminales legacy) | paleta | paleta → `Ctrl+Shift+P`≈ + `Ctrl+Alt+P` robusto |
| `F11` | Menú de plugins | — | `plugin_menu` → `F11` |

**Precedencia de la línea de comandos (modo `cli`).** Cuando la CLI tiene texto, sus teclas de edición tienen prioridad sobre el keymap del panel, igual que en Far: `Ctrl+Y`, `Ctrl+K`, `Ctrl+S`/`Ctrl+D`, `Ctrl+E`/`Ctrl+X`, `Ctrl+Enter`, `Ctrl+F`, `Ctrl+[`/`Ctrl+]`, `Home`/`End`. Con la CLI vacía, esos chords van al panel; por eso `Ctrl+K` (modal de atajos) sigue funcionando.

**Notas sobre las teclas reubicadas:**
- **`Alt+<dígito>`.** Se mantiene para ir a la pestaña N: la quick search de NC se usa con letras, y Far no da significado propio a `Alt+<dígito>`. Si el usuario escribe un dígito en la quick search ya abierta, se añade a la búsqueda.
- **AltGr.** En Windows, `Ctrl+Alt` es AltGr. Con teclado español, alemán o polaco, algunos `Ctrl+Alt+<tecla>` producen un carácter (`@`, `€`, `ś`…) y no llegan como chord. Por eso toda acción reubicada sigue accesible desde la barra de menú (`F9`) y la paleta, y el validador marca como frágiles `Ctrl+Alt+{E, Q, M, 1-9}` (≈).

---

## 6. Plugins

### 6.1 Manifiesto v2 (compatible hacia atrás)

```toml
[[commands]]
id       = "toggle"                 # → plugin.git-blame.toggle
title    = "Toggle blame"           # o clave i18n del plugin: "@cmd_toggle"
category = "Git"
context  = "panels"                 # panels | editor | viewer

[commands.keys]                     # sugerencias por preset; "default" para el resto
default  = "Alt+Shift+B"
neovim   = "<leader>gb"
yazi     = "g b"
standard = "Ctrl+Alt+B"
```

- El viejo `[keybindings] "tecla" = "accion"` se lee como `commands` con `keys.default`, con aviso de deprecación.
- Al ejecutarse, `entry(self, args)` recibe `args = { command = "toggle" }` con el id, más `args[1]` por compatibilidad. Se corrige la documentación.

### 6.2 Integración en el resolver

- `Command::Plugin(id)` entra en la **capa de plugins**, entre preset y usuario. Los plugins tienen secuencias, leader, which-key, paleta y modal gratis.
- **Política de colisión, determinista:**
  1. Un chord del preset nunca se pisa. El atajo del plugin queda **sin asignar** y el reporte lo marca como "conflicto con `<acción>`". El modal lo muestra con ⚠ y ofrece reasignarlo.
  2. Entre plugins, gana el primero por orden alfabético de nombre, con el mismo marcado ⚠ para el que pierde.
  3. El usuario siempre puede forzar con `overrides`.
- **Prefijo reservado para plugins**, sugerido en la guía para minimizar choques:
  - neovim `<leader>p…`;
  - yazi `Alt+p` y luego una letra (secuencia);
  - norton y standard `Ctrl+Alt+X` y luego una letra (secuencia), porque `Ctrl+Alt+<letra>` ya aloja las extensiones de Pairee en norton (§5.8).
  - Los chords de plugins fuera del prefijo se aceptan si están libres.
- Se usa un único parser de chords para todo: el de `keybinds` + `normalize_user_chord` + `normalize_key_spec`, fusionados en `keybindings::chord`. Desaparece la duplicación con `plugin/manager/dialogs.rs`.
- Al instalar, desinstalar, activar o desactivar un plugin se reconstruye el `Keymap` en caliente (`Keymap::rebuild(&config, &plugins)`), y con eso desaparece el mapa global de `plugin/registry.rs`.
- **API Lua:**
  - `pairee.keymap.list()`: solo lectura, el `km` del roadmap;
  - `pairee.keymap.chord_for("plugin.x.cmd")`;
  - `pairee.emit(action, args)` despachado en el bucle de la app mediante una cola de `Command`, para todas las acciones (hoy solo hay `cd`/`focus`).
  - `on_key` sigue siendo observador.
- **Herramientas:**
  - `pairee developer check` valida los chords contra los 4 presets y lista conflictos;
  - el packager corrige el bug `keybindings`→`hooks`;
  - `pairee plugin info` muestra los atajos.

---

## 7. Modal "Atajos de teclado"

### 7.1 Comportamiento

- Acción `keyboard_shortcuts`. Reemplaza el overlay which-key actual, que es una sola superficie. El HUD de prefijo se mantiene.
- **Layout** (sobre `ListPopup`, que soporta scroll, cabecera y estilos por fila; `FilterListView` no hace scroll):

```text
┌ Atajos de teclado — preset: Neovim ◂ ▸ ───────────────────────────────┐
│ > filtro…                       [Paneles] Editor  Visor  Listas  Plugins │
│ ── Navegación ───────────────────────────────────────────────────────── │
│   j, ↓            Mover abajo                                          │
│   g g, Home       Ir al inicio                                         │
│ ● Ctrl+w h        Foco panel izquierdo            (modificado)          │
│ ── Plugins · git-blame ──────────────────────────────────────────────── │
│ ⚠ <leader>gb      Alternar blame                  conflicto: …          │
│ ≈ Ctrl+Shift+P    Paleta de comandos              requiere kitty        │
├────────────────────────────────────────────────────────────────────────┤
│ Enter ejecutar · F2 reasignar · Ins añadir · Del quitar · F8 restaurar │
│ F3 buscar por tecla · Tab contexto · Ctrl+←/→ preset · F9 exportar      │
└────────────────────────────────────────────────────────────────────────┘
```

- **Columnas:** chords (todos); acción localizada; origen (base, preset, plugin `<nombre>`, usuario); estado.
- **Marcas de estado:** `●` modificado por el usuario, `⚠` conflicto o desplazado, `≈` frágil, `∅` sin asignar.
- **Agrupado** por `Category`, con cabeceras. Se incluyen también las acciones **sin asignar**, para poder asignarles un atajo.
- **Filtro.** Usa `nucleo`, como la paleta, sobre el chord, la etiqueta y el id.
- **Buscar por tecla (`F3`).** Se pulsa un chord o una secuencia y la lista muestra qué hace en cada contexto. Es el "Record keys" de VS Code.
- **Reasignar (`F2`) / añadir (`Ins`).**
  - Modo captura: muestra la secuencia en vivo, `Enter` confirma, `Esc` cancela. Si se quiere asignar `Esc` o `Enter`, se escribe en modo texto con `Ctrl+T`.
  - Si el chord está ocupado: diálogo "`Ctrl+K` ya está asignado a *X* en *Paneles*. ¿Reemplazar / Añadir igual en otro contexto / Cancelar?".
  - Avisa si el chord es frágil.
- **Persistencia.** Escribe en `[keybindings.overrides.<preset>]`, o en `all` si se marca "para todos los presets" con un checkbox en el diálogo. Después reconstruye el `Keymap` en caliente.
- **`F8` restaurar.** Quita el override de esa fila. `Shift+F8` restaura todo el preset, con confirmación.
- **`Ctrl+←/→` preset.** Previsualiza otro preset sin aplicarlo. Con `Enter` sobre el título se activa.
- **`F9` exportar.** Guarda `keymaps/<nombre>.toml` con `extends = "<preset>"` y solo las diferencias. Así un usuario crea su propio preset sin duplicar.

### 7.2 Resto de superficies, todas desde el mismo `Keymap`

- **Paleta:** muestra el chord a la derecha y lista los comandos de plugins. El título se genera con el chord real.
- **Ayuda F1:** `keyboard_shortcuts.md` deja de listar teclas y explica conceptos. Su primera entrada abre el modal.
  - CLI `pairee keymap print --preset <p> --format md` genera tablas para docs y README, y CI verifica que no haya deriva.
- **Barra de F-keys:** las filas de editor y visor salen de sus capas.
- **Onboarding:** el selector de preset muestra 6 atajos clave de cada uno, como vista previa.
- **Settings → Interfaz:** el ciclo de presets se genera escaneando `keymaps/`, excluido `base`. Se añade la fila "Abrir atajos de teclado…".

---

## 8. Fases de implementación

Cada fase va en commits pequeños. Al final de cada una: tests verdes, `clippy -D warnings`, sin duplicados (jscpd), módulos < 500 LOC y funciones < 100 líneas.

### Fase 0 — Saneamiento (bugs §1.2)
- [x] Unificar el nombre del preset en `keybindings.preset`, migrar `settings.keybinding_preset` y corregir `every_keymap_binds_undo_and_redo`.
- [x] Unificar `include_str!` de los presets en un único módulo `keymaps::embedded`.
- [x] Sembrado no destructivo: si el archivo del usuario difiere del embebido, no se pisa. Se escribe `<name>.toml.new` y se avisa en el reporte.
- [x] Eliminar el `preset == "vim"` de `cli.rs` (se reemplaza en la fase 1 por `typing`).
- [x] Corregir el formato de tecla de los plugins con el normalizador común, y añadir desregistro al desinstalar.

### Fase 1 — Modelo de keymap v2
- [x] `trait Bindable` + `Category` + catálogo completo (`keybindings/catalog.rs`). Test: toda variante del enum catalogada (se lee de `actions.rs`) y etiquetada en `en`/`es`.
- [x] Claves i18n `action_<id>` y `category_<name>` en `lang/en.toml` y `lang/es.toml`; which-key y la paleta muestran etiquetas localizadas.
- [x] Loader con capas (`loader/{layers,assign,validate}.rs`): `extends` (con detección de ciclos), `""` = desasignar, orden determinista (`BTreeMap` + filas ordenadas por chord), última capa gana, `report.displaced`. Los presets incluidos pasan a `extends = "base"` sin cambiar ningún binding (verificado contra una captura); efecto lateral: `Alt+F3` vuelve a abrir el visor alternativo (`view_alt` se leía como alias de `view`).
- [x] `[options]`: `typing`, `leader` (expansión `<leader>` en forma separada o compacta) y `sequence_timeout` en el resolver, con expiración en el tick para cerrar el HUD de prefijo.
- [x] Validación de prefijos sobre el keymap final: un chord completo que es prefijo de otra secuencia es error.
- [x] `chord::fragility()` + `report.robustness` para acciones `essential` sin chord robusto. El protocolo kitty ya se activaba en `terminal/backend.rs`.
- [x] Config `[overrides.all]` / `[overrides.<preset>]` en `keybindings.toml` + migración de `custom_bindings`.
- [x] Pipeline de entrada (§3.4): modos `cli` / `type_ahead` / `commands`, foco explícito de la CLI (`focus_cli`), y las teclas `s`/`v` del flujo yazi pasan a ser una capa de ajustes (`sort_menu` / `view_mode_menu`) en lugar de código fijo.

### Fase 2 — Acciones nuevas (§4)
- [x] Portapapeles de archivos (`yank`/`cut`/`paste`/`paste_overwrite`/`paste_as_link`/`clear_clipboard`) sobre el motor de transferencia (undo y conflictos incluidos) + indicador en la línea de comandos. Pegar una copia en su misma carpeta conserva ambas; un corte pegado en su carpeta no hace nada.
- [x] `trash`/`delete_permanent` (el título de la confirmación dice adónde van), `select_all`/`unselect_all`/`visual_mode`. `toggle_select_and_down` no hace falta: `select_item` ya marca y baja (NC `Insert`, yazi `Space`).
- [x] Historial `history_back`/`history_forward` por panel, `go_home`/`go_root`, `half_page_up`/`half_page_down` (la página sigue siendo de 10 filas fijas; ajustarla a la altura real queda pendiente).
- [x] `find_in_panel`/`find_next`/`find_prev` + type-ahead + quick search `Alt+letra` (opción de preset `alt_quick_search`), con un único buscador de nombres compartido.
- [x] `focus_left/right_panel`, `copy_name`/`copy_name_no_ext`/`copy_dir_path`, `new_file`/`create` (undo con `MakeFile`, solo mientras el archivo siga vacío), `rename_basename`, `cycle_panel_view`/`cycle_sort`.

### Fase 3 — Contextos editor / visor / lista
- [ ] `EditorAction`, `ViewerAction`, `ListAction` con `Bindable`; mover las teclas fijas a las capas `[editor]`, `[viewer]`, `[list]`.
- [ ] Pass-through "global" por metadato, sin F12/Ctrl+Tab fijos.
- [ ] Filas de F-keys de editor y visor generadas desde las capas.

### Fase 4 — Presets
- [ ] `base.toml`, `standard.toml` (+ alias `vscode`/`modern`), `norton.toml` (con §5.8: reubicaciones, precedencia de la CLI, acciones `insert_*_to_cli`, `toggle_inactive_panel`, `toggle_keybar`, `cli_history_*`; quitar `has_outdated_layout`), `neovim.toml`, `yazi.toml` según §5, cada línea con comentario de criterio cuando sea *ext.* o desviación.
- [ ] Migración: `preset = "vscode"` → `standard`.
- [ ] Onboarding y Settings con los 4 presets.

### Fase 5 — Plugins (§6)
- [ ] Manifiesto `[[commands]]` + compatibilidad con `[keybindings]`.
- [ ] `Command::Plugin` en la capa de plugins, política de colisión y reconstrucción en caliente.
- [ ] Paleta y modal con comandos de plugins; `pairee.keymap.*`; `pairee.emit` completo vía cola.
- [ ] `developer check`, fix del packager y `plugin info`.
- [ ] Actualizar `plugin-dev-guide(.es).md`, `plugin-system-design(.es).md` y la plantilla.

### Fase 6 — Modal de atajos (§7)
- [ ] Popup `KeyboardShortcuts` sobre `ListPopup` con cabeceras de categoría, pestañas de contexto, filtro y marcas.
- [ ] Buscar por tecla, reasignar/añadir/quitar/restaurar con diálogo de conflicto, persistencia y reconstrucción en caliente.
- [ ] Previsualizar preset y exportar preset derivado.
- [ ] Paleta con chords; título dinámico.

### Fase 7 — Documentación y CI
- [ ] `pairee keymap print` + test de no-deriva de `help/*/keyboard_shortcuts.md` y README.
- [ ] Reescribir la ayuda (en/es) y añadir guías "Vengo de Norton / Neovim / yazi / Explorer" con las diferencias inevitables.
- [ ] CHANGELOG y nota de migración.

---

## 9. Tests (criterios de aceptación)

- Cada preset carga sin errores; los avisos permitidos son solo los de fragilidad *con* fallback.
- Cada acción `essential` tiene un chord robusto en cada preset y contexto.
- Fidelidad: una tabla de casos `(preset, secuencia) → Command` replica §5, por ejemplo:
  - `(neovim, "g g") → GoToTop`
  - `(yazi, "c c") → CopyPath`
  - `(norton, "Shift+F6") → Rename`
  - `(standard, "Ctrl+V") → Paste`
- `extends`: los ciclos dan error, `""` desasigna lo heredado y el orden es determinista (test con 100 cargas).
- Override: gana el usuario, el desplazado queda en el reporte, `overrides.<preset>` solo aplica a su preset.
- Modo de tecleo: en neovim, una letra sin binding no toca la CLI; en norton sí; en standard salta al archivo.
- Timeout de secuencia, HUD de prefijo y `Esc` que cancela.
- Plugins:
  - un chord con modificador dispara;
  - un conflicto con el núcleo queda marcado;
  - el orden entre plugins es determinista;
  - al desinstalar se desregistra;
  - el comando aparece en la paleta y en el modal.
- Modal: smoke TUI de reasignar → persistir → recargar config → el chord nuevo dispara y el viejo no.
- Editor y visor: una acción global reasignada funciona dentro del editor.

---

## 10. Decisiones abiertas (requieren confirmación)

| # | Decisión | Recomendación |
|---|---|---|
| D1 | Norton `F7`. | **Decidido (2026-10-09):** volver al original de NC/Far. `F7` = MkDir, `Shift+F6` = renombrar, renombrado múltiple → `Ctrl+Shift+F6`≈ + paleta. Hay que retirar el chequeo `has_outdated_layout`, que fuerza `rename = "F7"`. |
| D2 | Renombrar `vscode` → `standard`. | **Decidido (2026-10-09):** sí; `vscode` y `modern` quedan como alias. |
| D3 | Preset por defecto. | Mantener `norton` (identidad de Pairee) y que el onboarding pregunte con vista previa. |
| D4 | Overrides por preset o globales. | Ambos (`overrides.all` + `overrides.<preset>`). El modal por defecto escribe en el preset activo. |
| D5 | Editor modal tipo vim para el preset neovim. | Fuera de este plan: fase futura. Aquí solo se mapean las teclas básicas. |
| D6 | Prefijo reservado para plugins. | Recomendado, no obligatorio: `<leader>p` (neovim), `Alt+p` + letra (yazi), `Ctrl+Alt+X` + letra (norton y standard). |
| D7 | Norton `Alt+letra` y demás choques con NC. | **Decidido (2026-10-09):** se respetan todos los atajos de NC/Far; las extensiones de Pairee que chocaban se reubican según §5.8. |
