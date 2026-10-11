# Pairee — Plan: soporte de mouse donde el entorno lo permita

> **Fecha:** 2026-10-09
> **Rama de trabajo:** `master`
> **Objetivo:** que todo lo que hoy se hace con teclado pueda hacerse también con el puntero (clic, doble clic, clic derecho, rueda, arrastre) **cuando el terminal reporta eventos de mouse**, sin degradar la experiencia donde no los hay (consola Linux sin gpm, `TERM=dumb`, pipes, terminales que no los soportan) y sin robarle al usuario la selección de texto nativa cuando el mouse está apagado.

Cada ítem usa checkbox (`[ ]` pendiente / `[x]` hecho, con commit / `[-]` decisión de no realizarlo). Actualizar entre tarea y tarea.

---

## 1. Diagnóstico del estado actual

### 1.1 Qué hay hoy

| Pieza | Ubicación | Estado |
|---|---|---|
| Captura de mouse | `src/terminal/backend.rs:64` | `EnableMouseCapture` **siempre** al iniciar, sin mirar el setting ni el entorno. |
| Setting `mouse_support` (bool, default `true`) | `src/config/settings/mod.rs:32`, `config_rows/tabs_extra.rs:22` | Solo filtra eventos en `events.rs`; no activa/desactiva la captura. |
| Router de eventos | `src/app/app/events.rs:106-118` | Cadena `||` de tres manejadores; sin capas ni prioridad de popups. |
| Scrollbars (drag, salto, rueda sobre el contenido) | `src/app/app/scrollbar_mouse.rs`, `src/ui/scrollbar/*` | Funciona. Hit targets registrados por frame en `ScrollbarUiState`. La rueda solo funciona si la barra está visible (hay desborde). |
| Barra de pestañas (clic = activar, medio = cerrar) | `src/app/app/tab_mouse.rs`, `src/ui/tab_bar.rs` (`TabBarHits`) | Funciona. Segundo mecanismo de hit-testing, paralelo al de scrollbars. |
| Editor (clic, drag, Alt+drag bloque, rueda) | `src/app/screen_input/editor/mouse.rs` | Funciona. Usa un tercer mecanismo (`Cell<Viewport>` del editor). |
| Harness de tests | `src/test_harness/drive.rs:43` | Solo `click()` izquierdo. |

### 1.2 Huecos y bugs

1. **La captura ignora el setting.** Con `mouse_support = false` el terminal igual queda en modo captura: el usuario pierde la selección/copia nativa y no gana nada a cambio.
2. **Sin detección de entorno.** En consola Linux (`TERM=linux`), `TERM=dumb` o stdout no-TTY se emiten secuencias de captura inútiles.
3. **Cambiar el setting en caliente no hace nada** sobre el terminal.
4. **`execute_shell_command` (`src/app/actions/exec.rs:24-27`)** sale del alternate screen y del raw mode **sin** `DisableMouseCapture`: el programa hijo (`less`, `vim`, la shell) recibe basura de secuencias de mouse al mover el puntero. Revisar también el retorno (re-habilitar).
5. **Panel de archivos sin mouse**: no hay clic para mover el cursor, doble clic para entrar, marcar, ordenar por columna ni rueda cuando no hay scrollbar.
6. **Barra de menú, menús desplegables, barra de F-keys, CLI, popups, botones, listas y checkboxes**: sin mouse. La ayuda (`help/*/ssh_sftp.md`) ya habla de "click [Save]" y `features.md` de "clicking on the badge": documentación por delante del código.
7. **Tres mecanismos distintos de hit-testing** (scrollbar, tabs, editor) → duplicación; agregar más superficies copiando cualquiera de ellos violaría la regla de cero duplicación.
8. crossterm no reporta doble clic: hay que derivarlo.
9. **No hay arrastrar y soltar** de archivos (entre paneles, a carpetas, a pestañas).
10. **Los modales están fijos en el centro**: los 36 archivos de `src/ui/popup` y `src/ui/transfer` los ubican con `centered_rect`, `centered_rect_fixed` o `centered_rect_in` (`src/ui/popup/mod.rs:147-190`), y unos ~20 `Rect::new` sueltos a auditar. No guardan posición propia.
11. **El split entre paneles es fijo 50/50** (`src/ui/layout.rs:80`): no se puede redimensionar ni con teclado ni con mouse.
12. **No hay garantía de cobertura**: nada impide que un popup o control nuevo nazca sin mouse.

---

## 2. Diseño

### 2.1 Disponibilidad: `auto | on | off`

- `mouse_support: bool` pasa a `mouse: MouseMode` con `Auto` (default), `On`, `Off`. Migración: `serde(alias = "mouse_support")` + deserializador que acepta `true`→`Auto`, `false`→`Off` (no romper configs existentes).
- Nuevo `src/terminal/mouse.rs` con `fn mouse_available(env) -> bool` (puro, testeable con un `EnvProbe` inyectado):
  - stdin/stdout no son TTY → no.
  - `TERM` = `dumb` → no; `TERM` = `linux` (consola VT, crossterm no habla gpm) → no.
  - Windows: sí (crossterm usa `ENABLE_MOUSE_INPUT` de la consola; funciona en conhost y Windows Terminal).
  - Resto (xterm, tmux, screen, kitty, wezterm, alacritty, foot, SSH sobre cualquiera de ellos) → sí.
  - Override: variable `PAIREE_MOUSE=0|1` y flag `--no-mouse` en `launch_args.rs` (para diagnóstico o terminales que mienten).
- Efectivo = `On` → siempre; `Off` → nunca; `Auto` → `mouse_available()`.

### 2.2 Ciclo de vida de la captura

- `TerminalBackend` gana `mouse_capture: bool` y `fn set_mouse_capture(&mut self, on: bool) -> Result<()>` idempotente; `init()` recibe el valor efectivo; `restore()` y el panic hook desactivan solo si estaba activa.
- Al aplicar la configuración (diálogo de opciones) se llama `set_mouse_capture` con el nuevo efectivo.
- `execute_shell_command` y cualquier otra suspensión del TUI pasan por un único par `suspend()/resume()` del backend (hoy la secuencia está escrita a mano en `exec.rs`); así la captura se suelta y se recupera en un solo lugar.
- Nueva acción `toggle_mouse_capture` (sin atajo por defecto en los presets salvo el `standard`, a decidir) para soltar el mouse temporalmente y poder seleccionar texto con el terminal. Documentar además que la mayoría de terminales permiten `Shift`+arrastre para la selección nativa aun con captura.

### 2.3 Hit-testing unificado

Un solo registro por frame que reemplaza `TabBarHits` y la lista de targets de `ScrollbarUiState` (el estado de drag de `tui_scrollbar` se mantiene):

```rust
// src/ui/hit/mod.rs
pub enum Layer { Base, Bar, Popup, Overlay }      // orden de pintado = prioridad inversa
pub enum HitTarget {
    PanelRow { side: ActivePanel, index: usize },
    PanelHeader { side: ActivePanel, column: SortColumn },
    PanelPath { side: ActivePanel },
    Tab(TabId),
    Scrollbar(ScrollbarHitTarget),
    MenuTitle(usize),
    MenuItem { menu: usize, item: usize },
    FKey(u8),
    Cli { offset: usize },
    PopupRow { list: ListId, index: usize },
    PopupButton(ButtonId),
    PopupField(FieldId),
    PopupOutside,
    PopupTitle, PopupResize,          // mover / redimensionar el modal (§2.8)
    PanelSplitter,                    // divisor entre paneles (§2.9)
    DropZone(DropTarget),             // destinos de arrastrar y soltar (§2.7)
    Badge(BadgeId),
    EditorText, ViewerText,
}
pub struct HitMap { cells: RefCell<Vec<(Layer, Rect, HitTarget)>> }
impl HitMap {
    pub fn clear(&self);
    pub fn register(&self, layer: Layer, area: Rect, target: HitTarget);
    pub fn at(&self, x: u16, y: u16) -> Option<&HitTarget>; // capa más alta, último registrado
}
```

- `draw_ui` hace `state.hits.clear()` al empezar; cada widget registra lo que pinta (la geometría ya la calcula al renderizar → cero duplicación de layout).
- Los helpers del kit de popups (`src/ui/popup/kit`: `dialog_frame`, `list_popup`, `filter_list`, `input_box`, `marked_row`) registran filas, campos y botones automáticamente, así cada popup gana mouse sin código propio.
- El editor puede seguir con su `Cell<Viewport>` pero se registra como `EditorText` para que el router sepa a quién entregarle el evento.

### 2.4 Interpretación de gestos

`src/app/mouse/gesture.rs` convierte `MouseEvent` crudo en gestos:

- `Click { button, mods }`, `DoubleClick` (mismo botón + misma celda/fila, < `double_click_ms`, default 400), `Drag { from, to }`, `Wheel { delta }`, `Release`.
- `Moved` se descarta salvo para hover del menú (y no marca la UI sucia si el target no cambió → sin redibujos por cada movimiento).
- Reloj inyectable para tests.

### 2.5 Router: el mouse produce `Action`s

`src/app/mouse/router.rs` reemplaza la cadena `||` de `events.rs`:

1. Si hay drag de scrollbar en curso → scrollbar.
2. `hits.at(x, y)` → target.
3. Si hay popup modal y el target no es de capa `Popup`/`Overlay`: clic = cerrar si el popup es descartable (igual que `Esc`), si no, se ignora.
4. Target → **`Action` existente** siempre que exista (`Enter`, `ToggleMark`, `SortBy(..)`, `OpenMenu(i)`, F-keys vía `key_for_action`, …) y se despacha con `handle_action`. Solo lo que no tiene acción (mover el cursor a una fila concreta, posicionar el cursor en un campo) se aplica directo con funciones compartidas con el teclado (`list_nav.rs`, `text_input`).

Así no se duplica lógica de negocio: el mouse es otro *front-end* de las mismas acciones.

### 2.6 Comportamiento por superficie

| Superficie | Clic izq. | Doble clic | Clic der. | Rueda / drag |
|---|---|---|---|---|
| Fila de panel | activa panel + mueve cursor | `Enter` (abrir/entrar) | según `mouse_right_click`: `mark` (Norton/Far, default) o `menu` (menú contextual) | rueda: 3 filas; `Ctrl`+clic marca; `Shift`+clic marca rango; drag der. marca rango (Far) |
| Encabezado de columna | ordenar por esa columna (repetir = invertir) | — | — | — |
| Ruta / título del panel | historial de carpetas | — | selector de unidad | — |
| Barra de pestañas | activar (ya existe) | renombrar | menú de pestaña | medio = cerrar (ya existe); rueda = siguiente/anterior |
| Barra de menú | abre/cierra menú | — | — | hover cambia de menú si ya hay uno abierto |
| Ítem de menú / menú contextual / user menu | ejecuta | — | — | hover resalta |
| Barra de F-keys | la acción de esa tecla | — | — | — |
| CLI | foco + posiciona cursor | selecciona palabra | — | — |
| Listas en popups | selecciona | acepta | — | rueda |
| Botones / checkboxes / radios | activa | — | — | — |
| Campos de texto | foco + cursor | selecciona palabra | — | drag selecciona |
| Fuera de popup | cierra si es descartable | — | — | — |
| Visor / QuickView | — | — | — | rueda siempre (no solo con scrollbar) |
| Git panel / transfer panel | selecciona fila | acción por defecto | — | rueda |
| Badge de actualización | abre diálogo de update | — | — | — |

Settings nuevos (con fila en Opciones → Interfaz, claves en `lang/en.toml` y `lang/es.toml`): `mouse` (auto/on/off), `mouse_right_click` (mark/menu), `double_click_ms`, `wheel_lines`, `drag_drop` (on/off), `drag_drop_confirm` (bool), `panel_split` (porcentaje, persistido).

### 2.7 Arrastrar y soltar archivos

Máquina de estados en `src/app/mouse/drag.rs` (`Idle → Pressed{origen} → Dragging{payload, sobre} → Drop | Cancel`):

- **Inicio:** `Down(Left)` sobre una fila del panel + `Drag` que se aleja ≥ 1 celda. Payload = los marcados si la fila de origen está marcada; si no, solo esa fila. `Esc` o soltar sobre algo no válido cancela.
- **Feedback mientras se arrastra:** etiqueta junto al puntero (capa `Overlay`) con nombre o "N elementos" y la operación (`Copiar` / `Mover` / `Enlazar`); el destino válido bajo el puntero se resalta y el cursor del panel destino lo sigue. Cerca del borde superior/inferior del panel, autoscroll.
- **Operación según modificadores** (como Total Commander/Explorer): sin modificador = copiar; `Shift` = mover; `Ctrl+Shift` = crear enlace (si `fs_ops/link.rs` lo admite en ese VFS). Si el terminal no reporta modificadores en el drag, se decide al soltar con un menú de 3 opciones.
- **Destinos:**

| Soltar sobre | Destino |
|---|---|
| Área del otro panel (fila de archivo, vacío o encabezado) | directorio actual del otro panel |
| Fila de una carpeta (en cualquier panel) | esa carpeta |
| Pestaña de la barra de pestañas | directorio de esa pestaña |
| Ítem del hotlist / acceso directo de carpeta, si está visible | esa ruta |
| CLI | inserta las rutas (con comillas `shlex`) |
| Mismo directorio de origen | nada (cancelar) |

- **Ejecución:** el drop **no** copia por su cuenta: abre el mismo prompt de F5/F6 (`TransferPromptOp`, `src/app/actions/fs_ops/transfer.rs`) con el destino ya completado, o lo confirma directo si `drag_drop_confirm = false`. Así reutiliza filtros, conflictos, cola de transferencias, deshacer y SFTP (cualquier VFS), sin duplicar lógica.
- **Arrastre desde el sistema operativo:** soltar archivos del Explorador/Finder en la ventana del terminal llega como *paste* de rutas. `handle_paste` detecta rutas existentes y ofrece copiar o mover al panel activo (o pegar en la CLI si tiene foco).
- **Límite:** arrastrar *desde* Pairee *hacia* otras apps no es posible en un TUI. Documentarlo.
- Conflicto de gestos resuelto: arrastre izquierdo en panel = drag & drop; arrastre derecho = marcar rango (§2.6).

### 2.8 Modales movibles (y redimensionables)

- Cada entrada de `DialogStack` (`src/app/state/dialog_stack.rs`) guarda `placement: Placement { offset: (i16, i16), size: Option<(u16, u16)> }`; se pierde al cerrar el popup y se conserva mientras está suspendido (`suspended_popup`).
- Las tres funciones `centered_rect*` pasan a un único `place(desired, screen, placement) -> Rect` que centra, aplica offset/tamaño y **acota a la pantalla** (el modal nunca queda fuera ni pierde el título). Los ~20 `Rect::new` sueltos de popups se auditan y se llevan a `place` → un solo punto, y todos los modales se mueven sin tocar cada uno.
- El borde superior (título) de cada modal se registra como `HitTarget::PopupTitle`: arrastrarlo mueve el modal; doble clic en el título lo recentra.
- Esquina inferior derecha como `HitTarget::PopupResize` solo en modales con contenido elástico (visor, help, QuickView, git panel, diff, transfer panel, editor de popup): arrastrar cambia el tamaño, con mínimo por popup.
- Teclado (paridad, mismas acciones): `popup_move` / `popup_recenter` en el catálogo, sin atajo por defecto obligatorio.
- Opcional: recordar posición/tamaño por tipo de popup en la sesión (`src/app/session`).

### 2.9 Divisor entre paneles

- `panel_split` (default 50) reemplaza el 50/50 fijo de `layout.rs`; el borde entre paneles se registra como `HitTarget::PanelSplitter`: arrastrar cambia el porcentaje (mín. 15 %, máx. 85 %), doble clic vuelve a 50.
- Acciones de teclado `panel_split_grow` / `panel_split_shrink` / `panel_split_reset`.

### 2.10 "Todo interactuable": inventario y garantía

Criterio de aceptación: **todo elemento que se pueda accionar con teclado se puede accionar con el mouse**. Para que no dependa de memoria:

1. **Inventario** (tabla en este documento, actualizada por fase) de cada `PopupType`, pantalla y barra, con su estado de mouse (listas, campos, botones, checkboxes/radios, pestañas internas de diálogos como `config_dialog`, títulos, scroll).
2. **Controles del kit con mouse incorporado**: botón, checkbox, radio, selector cíclico (`< valor >`: clic en flechas o rueda), campo de texto, lista, pestañas internas, árbol (`tree_view`: clic en `+/-` expande). Un popup que use el kit no puede quedar sin mouse.
3. **Test de cobertura** en CI: recorre todas las variantes de `PopupType` y `Screen` (con constructores de muestra), las renderiza en `TestBackend` y falla si alguna no registra hit targets para cada control accionable que pinta. Agregar una variante nueva sin mouse rompe el test.
4. Menús: barra superior, desplegables, submenús (`panel_submenus.rs`), menú contextual, user menu, screens menu, plugin menu, command palette, which-key y menús de plugins Lua.

---

## 3. Fases y tareas

### Fase 0 — Disponibilidad y captura (corrige bugs 1–4)
- [x] `MouseMode` + migración serde de `mouse_support`; actualizar defaults, fila de config, `lang/*`, settings-helper.
- [x] `src/terminal/mouse.rs::mouse_available` con `EnvProbe`; tests por `TERM`/TTY/override.
- [x] `--no-mouse` y `PAIREE_MOUSE`.
- [x] `TerminalBackend::set_mouse_capture` + `suspend()/resume()`; `init`, `restore`, panic hook.
- [x] `exec.rs` usa `suspend/resume` (fin de secuencias de mouse en procesos hijos).
- [x] Aplicar el cambio de setting en caliente.
- [x] Acción `toggle_mouse_capture` + entrada en catálogo de acciones.

### Fase 1 — Infraestructura
- [x] `ui::hit::HitMap` + `Layer` + `HitTarget`; `state.hits`; `clear` al inicio de `draw_ui`.
- [x] Migrar `TabBarHits` y los targets de scrollbar a `HitMap` (borrar el código viejo, tests existentes en verde).
- [x] `gesture.rs` (doble clic, drag, filtrado de `Moved`) con reloj inyectable.
- [x] `router.rs` con política modal; `events.rs` delega en él.
- [x] Harness: `right_click`, `middle_click`, `double_click`, `wheel`, `drag`, `hover`.

### Fase 2 — Paneles
- [x] Registrar filas en vistas brief y tabla (`src/ui/panel/*`, `list_ctx.rs`), encabezados y ruta.
- [x] Clic / doble clic / `Ctrl`/`Shift`+clic / clic der. / drag der. / rueda.
- [x] Rueda sin depender de que la scrollbar esté visible.

### Fase 3 — Barras
- [x] Menú superior y desplegables (`src/ui/menu/*`, `input_popup/menu.rs`), hover. Los desplegables se ubican bajo su título (antes, columnas fijas que no seguían la traducción). Con la barra oculta, la fila superior la revela al apuntar (como Far); las pestañas de esa fila conservan sus clics.
- [x] Barra de F-keys (`src/ui/fkeys.rs`): el clic pulsa la tecla con los modificadores de la fila mostrada, así sirve en panel, editor y visor.
- [x] CLI (`src/ui/cli.rs`), barra de transferencias, badge de update.
- [-] **Decisión: NO realizar.** Rueda en la barra de pestañas: no queda claro qué lado ni desde qué pestaña avanzar.

### Fase 4 — Popups vía kit
Enfoque elegido al implementar: **el mouse maneja cada diálogo con sus propias teclas**. El renderer sabe dónde está el foco o el cursor, así que cada control registra las teclas que lo alcanzan y lo usan desde ahí (`HitTarget::PopupKeys`, `PopupRow { steps, focus }`, `PopupField { keys, text_x }`), y el router las inyecta por el manejador de teclado del diálogo. No hay código de mouse por popup ni estado duplicado.
- [x] Kit: `ListPopup` (18 diálogos) y `FilterListView` registran filas; `kit::mouse::FormFocus` + `form_buttons` / `form_labelled_field` / `form_inline_field` / `form_option` registran campos, opciones y botones de los formularios con `FormLayout`.
- [x] Formularios: crear carpeta, renombrar, copiar/mover, buscar en editor/visor, buscar archivos, opciones de sincronización, renombrado múltiple, prompts de Git (nombre, dos campos), conexión SSH (incluida la lista de presets).
- [x] Hechos a mano: confirmar borrado, diálogos de plugins (sí/no, lista "which"; además se corrigió el cuerpo registrado cuando el plugin elige la posición), confirmaciones de Git, actualización, ayuda (pestañas + lista), configuración (pestañas por su tecla rápida, filas saltando títulos, OK/Cancelar, edición en curso), panel de Git (pestañas + filas), menú de usuario, modos de orden (clic = aplicar), bienvenida, grupos de color y resaltado (filas + `<` `>`), asociaciones, remotos de Git, gestor de plugins (pestañas, instalados, desarrollo), pestañas del panel de transferencias, atajos (filas con encabezados de categoría + pestañas), popups Yazi (cuerpo + celdas), atributos (solo lectura).
- [x] Test de cobertura `every_dialog_records_its_body_and_its_controls`: abre 35 diálogos con sus acciones reales y exige cuerpo y al menos un control.
- [x] Clic en el campo de los 11 prompts de un solo campo (`kit::TextBox::with_field` + `kit::field_at`, que calcula fila y columna del campo en la frase traducida).
- [x] Pestaña Buscar del gestor de plugins: filas clicables y el indicador de página pasa de página (se corrigió además que ese indicador nunca se veía: la página no reservaba la fila de títulos).
- [x] Lista de trabajos del panel de transferencias: cada trabajo (tres filas) toma el clic.

### Fase 5 — Visor, QuickView, editor
- [x] Rueda en visor siempre, también con la scrollbar apagada (`scrollbar::register_wheel`); QuickView ya la tenía. El editor sigue con su manejo propio, como último destino del router.

### Fase 5b — Modales movibles y divisor de paneles
- [x] `ui::placement`: las funciones de centrado aplican la ubicación del diálogo abierto (`place`), sin tocar cada popup. La ubicación vale para ese diálogo (profundidad + tipo) y se pierde al cerrarlo. Se unificaron `centered_rect` y `centered_rect_in`, que eran iguales.
- [x] Arrastrar el borde superior mueve, doble clic recentra; siempre dentro de la pantalla, y al volver desde el borde responde enseguida.
- [x] Redimensionar desde la esquina en los diálogos de contenido elástico (`PopupType::resizable`).
- [-] **Decisión: NO realizar.** Acciones de teclado `popup_move` / `popup_recenter`: mover un diálogo con teclas pediría un modo propio y el valor es bajo.
- [x] `panel_split` + `PanelSplitter` (arrastre relativo al agarre, doble clic = 50 %) + acciones `panel_split_grow` / `panel_split_shrink` / `panel_split_reset`.

### Fase 5c — Arrastrar y soltar
- [x] `app::mouse::file_drag`: un clic izquierdo simple sobre una fila queda pendiente; al moverse una celda empieza el arrastre (marcados si la fila está marcada, si no esa fila; nunca `..`). `Esc` o cualquier tecla lo cancela.
- [x] Etiqueta junto al puntero (`ui::drag_label`): "Copiar alpha.txt → destino", o solo el nombre cuando ahí no se puede soltar.
- [x] Destinos: el otro panel (filas, vacío, título, encabezados), una carpeta de cualquier panel, `..` (carpeta padre), una pestaña, la línea de comandos (rutas entre comillas). Soltar donde ya están los archivos, o una carpeta dentro de sí misma, no hace nada.
- [x] Sin modificador copia; con `Shift` al soltar mueve (Shift al presionar marca un rango). Se ejecuta con `fs_ops::transfer::start`, el mismo camino de F5/F6 (diálogo o envío directo según la confirmación); el diálogo guarda las sesiones SSH de origen y destino (`CopyMovePromptState::endpoints`), así funciona también al soltar en una subcarpeta del mismo panel SFTP.
- [-] **Decisión: NO realizar.** Enlazar con `Ctrl+Shift` y el menú de respaldo cuando el terminal no informa modificadores: Copiar/Mover cubren el uso y el diálogo deja cambiar el destino.
- [x] Autoscroll al acercarse al borde de un panel: con el puntero en la primera o última fila visible, la vista de ese panel avanza una fila (al moverse y cada 100 ms con el temporizador) sin mover su cursor; al soltar o cancelar vuelve a seguir al cursor (`FileDrag::peek`).
- [-] **Decisión: NO realizar.** Convertir en copia las rutas soltadas desde el Explorador/Finder: el terminal las entrega como pegado y siguen yendo a la línea de comandos; convertirlas rompería pegar rutas en la CLI.

### Fase 6 — Plugins
- [x] Evento Lua `on_mouse` (`{kind, button, x, y, target, side?, index?}`) para clics en paneles y barras (no dentro de diálogos, igual que `on_key`), documentado en las guías de plugins y en `docs/api/lua/CHANGELOG.md`.

### Fase 7 — Documentación y cierre
- [x] `help/{en,es}/features.md` (sección 1.5 Ratón), `configuration_details.md` (modo, clic derecho, doble clic), `user_guide.md` (`--no-mouse`).
- [x] `docs/UNRELEASED.md` (Added / Fixed).
- [x] `docs/technical/architecture_{en,es}.md`, sección 11.

## 4. Pruebas

- Unitarias: `mouse_available` (matriz de entornos), `MouseMode` serde (bool viejo → enum), gestos (doble clic dentro/fuera del umbral, celda distinta), `HitMap::at` (prioridad de capas, solapes).
- Integración con `test_harness` + `TestBackend`: clic en fila mueve cursor; doble clic entra a carpeta; clic der. marca; clic en encabezado ordena; clic en F-key ejecuta; menú se abre y ejecuta ítem; clic fuera cierra popup descartable y no cierra uno modal; rueda en panel sin scrollbar; `mouse = off` ignora todo.
- Drag & drop: arrastrar fila al otro panel abre el prompt de copia con destino correcto; `Shift` = mover; soltar sobre carpeta usa esa carpeta; soltar en el mismo directorio no hace nada; `Esc` cancela; payload = marcados vs. fila única.
- Modales: arrastrar título desplaza y acota a pantalla; `Resize` del terminal no deja el modal fuera; cerrar y reabrir vuelve al centro; redimensionar respeta mínimos.
- Cobertura: el test de §2.10 debe fallar si se agrega un `PopupType` sin hit targets.
- Manual (checklist en el PR): Windows Terminal, conhost, tmux, SSH a Linux, consola VT Linux (`auto` debe quedar sin captura), kitty/wezterm, macOS Terminal/iTerm2.

## 5. Riesgos

- **Flood de `Moved`**: crossterm activa reporte de movimiento total; filtrar temprano y no redibujar si nada cambió.
- **Selección nativa**: mitigada con `toggle_mouse_capture`, `Shift`+arrastre y `mouse = off`.
- **Conhost QuickEdit**: crossterm lo desactiva al capturar; verificar que `restore()` lo devuelve como estaba.
- **Modificadores en el drag**: algunos terminales no los reportan en eventos `Drag` → menú de respaldo al soltar.
- **Drop accidental**: umbral de 1 celda para iniciar el drag y confirmación por defecto (`drag_drop_confirm = true`).
- **Desfase frame/evento**: los hits son del último frame pintado; aceptable (mismo modelo que scrollbars hoy). Si se resuelve un target obsoleto, la acción valida índices antes de aplicar.
