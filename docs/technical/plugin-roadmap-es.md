# Sistema de Plugins de Pairee — Roadmap de Evolución

> **Documento de diseño interno de Pairee. Enumera las brechas del sistema de plugins actual, especifica la nueva superficie de runtime (userdata tipado, contextos sync/async, builder de `Command`, widgets UI, diálogos, utils) y plantea un plan de implementación por fases (M0–M5).**

---

## Tabla de Contenidos

1. [Resumen Ejecutivo](#1-resumen-ejecutivo)
2. [Inventario del Estado Actual](#2-inventario-del-estado-actual)
3. [Inventario de Brechas](#3-inventario-de-brechas)
4. [Fundamentos Arquitectónicos](#4-fundamentos-arquitectónicos)
5. [Superficie de la Nueva API por Área](#5-superficie-de-la-nueva-api-por-área)
6. [Mapeo de Secure Mode para las Nuevas APIs](#6-mapeo-de-secure-mode-para-las-nuevas-apis)
7. [Migración y Compatibilidad Hacia Atrás](#7-migración-y-compatibilidad-hacia-atrás)
8. [Plugins de Aceptación](#8-plugins-de-aceptación)
9. [Roadmap de Implementación](#9-roadmap-de-implementación)
10. [Apéndice A — Mapeo de Pairee Actual → Propuesto](#apéndice-a--mapeo-de-pairee-actual--propuesto)
11. [Apéndice B — Archivos de Pairee Afectados](#apéndice-b--archivos-de-pairee-afectados)
12. [Apéndice C — Material de Referencia](#apéndice-c--material-de-referencia)

---

## 1. Resumen Ejecutivo

Pairee envía un sistema de plugins Lua funcional y en sandbox, construido sobre `mlua`. La superficie actual (`pairee.app`, `pairee.fs`, `pairee.ui`, `pairee.ps`, `pairee.log`, `pairee.sync`) cubre los casos de uso básicos — previewers, hooks, comandos, pub/sub, settings, i18n, ayuda F1, un asistente TUI de desarrollo, y un registro con distribución por SHA-256.

Sin embargo, la superficie actual está **incompleta a nivel de runtime**: los valores que reciben los plugins son tablas Lua planas sin métodos, sin metamétodos, y sin metadatos ricos; cada plugin corre en un hilo worker con un snapshot one-shot del estado; los primitivos de diálogo son stubs y nunca muestran un diálogo real; el builder `Command` no existe (solo hay un `spawn` fire-and-forget); el conjunto de widgets UI son seis constructores de tablas planas; el estado vivo (`cx`), el acceso al tema (`th`), las preferencias (`rt`), y el acceso al keymap (`km`) están completamente ausentes; y los métodos `preload`/`seek` documentados en la guía de desarrollo no están realmente enrutados.

Este documento enumera **14 brechas específicas** en la implementación actual (cada una citada con evidencia `file:line`), especifica la nueva superficie de runtime propuesta en **6 áreas** (userdata tipado, fs async + Command, widgets UI ricos, contexto vivo, diálogos reales + despacho de acciones + utils, modelo sync/async con anotaciones) y plantea un **plan de implementación en 6 fases (M0–M5)** de 1–3 semanas cada una — totalizando ~10–14 semanas de trabajo para un único desarrollador Rust+Lua.

El material de referencia en `example/reference/` (un sistema de plugins de terceros en producción, vendorizado) se usó para validar el diseño propuesto y para obtener patrones concretos de las características más avanzadas (userdata tipado, bridge sync/async, builder de widgets, streaming de comandos). Ningún nombre de tercero se expone en la API pública de Pairee ni en este documento.

---

## 2. Inventario del Estado Actual

Esta sección lista todo lo que el sistema de plugins actual de Pairee expone y enruta realmente, basado en una lectura completa de `src/plugin/`. Cada entrada cita el archivo fuente y la línea.

### 2.1 Motor y ciclo de vida

| Concepto | Estado | Ubicación |
|---|---|---|
| Descubrimiento del directorio de plugins | Implementado | `src/plugin/manager.rs:107-160` escanea `~/.config/pairee/plugins/` buscando carpetas `*.pairee` |
| Carga del manifest (`manifest.toml`) | Implementado | `src/plugin/loader.rs:24-35` (soporta tabla plana o anidada `[plugin]`); `:69-119` lee + parsea + chequea versión |
| Creación del sandbox | Implementado | `src/plugin/sandbox.rs:46-77` (`create_sandboxed_lua`) — `StdLib::TABLE \| STRING \| UTF8 \| MATH`, elimina `load/loadstring/dofile/loadfile`, `require` custom delimitado por ruta |
| Flag de modo trusted | Implementado | `src/plugin/loader.rs:85`, `:150` (no se ejecuta `tokio::process::Command` cuando `trusted=false`) |
| Secure Mode | Implementado | `src/plugin/sandbox.rs:5-44` (blacklist de 27 comandos); `src/plugin/runtime/bindings/fs.rs:13-33` (FS restringido a workspace + config + cache) |
| Chequeo de versión mínima | Implementado | `src/plugin/loader.rs:78-82, 122-152` (parser custom de semver, sin granularidad `@since`) |
| Canales de contexto del plugin | Implementado | `src/plugin/manager.rs:87-104` (mpsc send/receive, buffer de 100) |
| VM Lua por plugin | Implementado | `src/plugin/sandbox.rs:46-77` (un `mlua::Lua` por plugin) |
| Teardown del plugin | Parcial | `src/plugin/registry.rs:78-99` (canal de tarea por plugin), sin API global de descarga |

### 2.2 Métodos de plugin actualmente enrutados

`src/plugin/registry.rs:17-29` define solo tres variantes de `PluginTaskRequest`:

| Variante | Propósito | Notas |
|---|---|---|
| `Peek { job, reply_tx }` | Renderizado de preview | El job lleva `file_path, area_width, area_height, skip` (sin userdata `File`, sin `mime`) |
| `ExecuteCommand { args }` | Invocación de plugin de comando | Enrutado vía `run_command` |
| `EmitEvent { name, data }` | Entrega de pub/sub | Payload de string JSON |

`src/plugin/registry.rs:104-149` (`execute_peek_internal`) construye una tabla Lua con solo `file.url` y `file.path` para el job de `peek` — **sin `cha`, sin `mime`, sin `is_hidden`, sin `is_exec`, sin timestamps, sin `perm()`**.

### 2.3 Globals Lua actualmente expuestos

`src/plugin/runtime/standard.rs` y `src/plugin/runtime/bindings/*.rs` registran:

| Ruta Lua | Estado | Notas |
|---|---|---|
| `pairee.app.cwd` | Lectura (snapshot) | `src/plugin/runtime/bindings/app.rs:9-29` lee de `_current_snapshot` |
| `pairee.app.cd` | Escritura | `app.rs:31-42` (envía `PluginRequest::Cd`) |
| `pairee.app.focus` | Lectura (snapshot) | `app.rs:45-60` |
| `pairee.app.set_focus` | Escritura | `app.rs:63-73` (envía `PluginRequest::SetFocus`) |
| `pairee.app.notify` | Escritura (real) | `app.rs:75-86` → `manager.rs:191-194` abre un `PopupType::Info` real |
| `pairee.app.confirm` | **STUB** | `app.rs:88-111` → `manager.rs:207-215` siempre devuelve `true` |
| `pairee.app.input` | **STUB** | `app.rs:113-136` → `manager.rs:215-223` siempre devuelve `default` |
| `pairee.app.hovered` | Lectura (snapshot) | `app.rs:139-153` |
| `pairee.fs.read` | Sync, bloqueante | `bindings/fs.rs:43-50` usa `std::fs::read_to_string` |
| `pairee.fs.write` | Sync, bloqueante | `bindings/fs.rs:52-60` usa `std::fs::write` |
| `pairee.fs.exists` | Sync, bloqueante | `bindings/fs.rs:62-69` |
| `pairee.fs.stat` | Sync, bloqueante | `bindings/fs.rs:71-96` |
| `pairee.fs.list` | Sync, bloqueante | `bindings/fs.rs:98-127` (sin opciones glob/limit/resolve) |
| `pairee.fs.spawn` | Fire-and-forget | `bindings/fs.rs:130-161` (sin streaming, sin stdin, sin env, sin kill) |
| `pairee.fs.spawn_copy_task` | Async (atado a UI) | `bindings/fs.rs:163-176` (no hay `copy`/`mkdir`/`remove`/`rename` planos) |
| `pairee.ui.Paragraph` | Solo tabla | `bindings/ui.rs:4-12` devuelve tabla plana |
| `pairee.ui.Gauge` | Solo tabla | `bindings/ui.rs:14-23` |
| `pairee.ui.List` | Solo tabla | `bindings/ui.rs:25-33` |
| `pairee.ui.Table` | Solo tabla | `bindings/ui.rs:35-46` |
| `pairee.ui.Span` | Solo tabla | `bindings/ui.rs:48-57` |
| `pairee.ui.Line` | Solo tabla | `bindings/ui.rs:59-67` |
| `pairee.ps.sub/pub/unsub` | Implementado | `bindings/ps.rs:5-60` (tabla `_callbacks` por VM, solo local) |
| `pairee.log.{info,warn,error,debug}` | Implementado | `bindings/log.rs:1-37` |
| `pairee.sync(fn)` | Bridge | `bindings/sync.rs:5-57` (solo snapshot oneshot) |
| `pairee.settings.*` | Read-only del manifest | `standard.rs:42-107` (`settings_schema` parseado del `manifest.toml`) |
| `pairee.t(key, vars)` | i18n | `standard.rs:109-191` (lang TOML, fallback a corchetes) |

### 2.4 Fortalezas actuales a preservar

- **Tooling de autor** (sin parangón en sistemas comparables):
  - `settings_schema` del manifest renderizado en la TUI de configuración
  - i18n por plugin vía `lang/<locale>.toml`
  - Integración de ayuda F1 vía `help/<locale>.md`
  - Rama huérfana `plugin-template` con sustitución de placeholders
  - CLI `pairee developer` (`format`, `validate`, `package`) con reglas estrictas de naming y encoding cross-platform
  - Asistente TUI Developer Tools (init, audit, package, install, submit)
  - Registro con verificación SHA-256 archivo por archivo y suite CLI (`search`, `list`, `install`, `update`, `check-updates`, `pin`, `remove`)
- **Sandboxing**: más estricto que el típico — eliminación explícita de `load/loadstring/dofile/loadfile` más `require` custom (sin `io`/`os`/`package`/`debug`/coroutine en untrusted)
- **Secure Mode**: blacklist de 27 comandos + frontera de FS — más allá de lo que la mayoría de sistemas comparables ofrecen

---

## 3. Inventario de Brechas

Cada brecha se cita con evidencia `file:line`. Los números (G1–G14) se usan a lo largo del resto del documento.

### 3.1 Sin userdata tipado — G1

`src/plugin/manager.rs:18-26` define `FileEntrySnapshot { name, url, path, size, is_dir, is_symlink }` como un struct plano serializable. Cuando se envía a Lua, se convierte en una tabla plana (`src/plugin/registry.rs:117-124` construye una tabla con solo `url` y `path` — sin `cha`, sin `mime`, sin `is_hidden`, sin `is_exec`, sin timestamps, sin `perm()`).

Los plugins reciben una tabla delgada y deben derivar todo lo demás por sí mismos. No hay forma de añadir métodos, ni metamétodos (`__eq`, `__tostring`, `__concat`), ni forma de extender el tipo desde fuera del núcleo de Rust.

### 3.2 Sin contexto sync / sin estado vivo — G2

`src/plugin/runtime/bindings/sync.rs:8-56` es el único bridge. No hay un global equivalente a un contexto vivo (llamémoslo `cx`). Los plugins leen `pairee.app.cwd()` (que lee de `_current_snapshot` establecido por `sync.rs:41`) pero no pueden iterar el panel, listar archivos seleccionados, obtener la posición del cursor, leer el progreso de tareas, o consultar el otro panel.

Esta es la brecha única más profunda: cada característica UI avanzada (previewers que dependen del cursor, hooks que dependen de la selección, paneles pluggables) está bloqueada.

### 3.3 Sin builder `Command` — G3

`src/plugin/runtime/bindings/fs.rs:130-161` es un único `spawn(cmd, args)` que llama a `tokio::process::Command::new(&cmd).args(&args).output().await` y devuelve `{stdout, stderr, status}`. Sin stdin, sin env, sin cwd, sin streaming, sin kill, sin distinción INHERIT/PIPED/NULL. Sin objeto `Child`. Sin userdata `Output`/`Status`.

Esto por sí solo impide toda la clase de plugins interactivos (buscadores difusos, navegadores de historial, integraciones de REPL) que dependen de control bidireccional de stdin/stdout.

### 3.4 `fs.*` es sync-bloqueante y mínimo — G4

`src/plugin/runtime/bindings/fs.rs:43-127` usa `std::fs::read_to_string`, `std::fs::write`, `std::fs::metadata`, `std::fs::read_dir` — **I/O síncrono dentro de `create_function`/`create_async_function`**. Estos bloquean el hilo worker del plugin (e indirectamente el worker del runtime tokio si se programa en uno). Faltan: `mkdir`, `remove`, `rename`, `copy` (solo existe `spawn_copy_task`, que está atado a UI), `read_dir({glob, limit, resolve})`, builder `Access`/`Fd`, errores tipados.

### 3.5 Los widgets UI son stubs de tabla — G5

`src/plugin/runtime/bindings/ui.rs:1-69` expone 6 constructores que devuelven tablas planas con un discriminador `type=...`: `Paragraph`, `Gauge`, `List`, `Table`, `Span`, `Line`. Los valores de widgets se devuelven al hilo principal vía `PluginRequest::UpdatePluginWidget { path, widget }` y se deserializan por serde a `PluginWidget` (`src/app/state/types/...`). Sin styling, sin layout, sin posicionamiento Rect, sin bordes, sin alineación, sin wrap, sin renderizado de imagen.

### 3.6 Pub/sub es solo local y superficial — G6

`src/plugin/runtime/bindings/ps.rs:5-60` usa una tabla Lua por VM `_callbacks` (línea 8) para almacenar suscripciones. La comunicación entre plugins se logra mediante `emit_event` en Rust (`src/plugin/hooks.rs:4-17`) que itera todos los plugins cargados y llama a la VM de cada uno. No hay bridge remoto, no hay capa de serialización, no hay equivalente a un sistema de entrega entre instancias, no hay `pub_to`. El `pairee.ps.unsub` (línea 47-57) establece la entrada del evento a `nil` pero no maneja el caso donde múltiples plugins se suscribieron al mismo evento.

### 3.7 `cd` y `set_focus` son los únicos puntos de entrada de despacho de acciones — G7

`src/plugin/runtime/bindings/app.rs:31-73` registra solo `cd(path)` y `set_focus(side)`. El enum `PluginRequest` (`src/plugin/manager.rs:42-85`) tiene 11 variantes, pero todas son operaciones de lectura (Notify, Cd, SetFocus, Confirm, Input) o de propósito especial (SpawnCopyTask, UpdatePluginWidget, PluginMenuLoaded, DevPluginScan, GetStateSnapshot). No hay una variante `EmitAction(name, args)` que permita a un plugin disparar cualquier enum `Action` de Pairee (definido en `src/keybindings/actions.rs`).

### 3.8 Los diálogos son stubs — G8

`src/plugin/manager.rs:207-223`:
```rust
PluginRequest::Confirm { title, msg, reply_tx } => {
    log::info!("Plugin confirm dialog requested: {} - {}", title, msg);
    let _ = reply_tx.send(true);  // <-- siempre true; el usuario nunca ve un diálogo
}
PluginRequest::Input { title, default, reply_tx } => {
    log::info!("Plugin input dialog requested: {} - {}", title, default);
    let _ = reply_tx.send(default);  // <-- siempre default
}
```

Ambos diálogos se loguean y se envía una respuesta enlatada. El usuario nunca ve un diálogo real. El path de `notify` (`manager.rs:191-194`) al menos establece `state.active_popup = Some(PopupType::Info(...))`, así que las notificaciones sí funcionan.

### 3.9 Preloader y seek sin enrutar — G9

`src/plugin/registry.rs:17-29` define solo `PluginTaskRequest::{ Peek, ExecuteCommand, EmitEvent }`. `Preload` y `Seek` están documentados en `docs/plugin-dev-guide.md:8` pero no implementados. El `PreviewJob` (`registry.rs:10-15`) solo lleva `file_path, area_width, area_height, skip` — sin userdata `file`, sin `mime`.

### 3.10 La cobertura de eventos es delgada — G10

`docs/plugin-dev-guide.md:38` lista solo `on_cd`, `on_hover`, `on_key`, `on_focus`. Un conjunto más rico de eventos internos (`reveal`, `select`, `toggle`, `yank`, `paste`, `update_yanked`, `update_mimes`, `update_files`, `tasks:update_succeed`, `update_peeked`, `update_spotted`) no se expone a los plugins.

### 3.11 El renderizado TUI de settings por plugin es parcial — G11

Pairee tiene `settings_schema` (`docs/plugin-dev-guide.md:18`) y una sección TUI que lo renderiza, pero el código de renderizado está en la UI de config más amplia, no en el módulo de plugins. La TUI está documentada pero la variante `PluginMenu` de `src/ui/popup.rs` (`src/plugin/manager.rs:256-268`) solo maneja instalación/actualización de plugins, no el renderizado de formularios de settings por plugin.

### 3.12 Sin preview de imagen — G12

`Cargo.toml:25` tiene el crate `image` (0.25.10), y el visor F3 del core renderiza imágenes, pero ninguna API Lua expone el preview de imagen. La pila de adaptadores de imagen de terminal (Chafa, iTerm2, Sixel, Überzug) tampoco se expone a los plugins.

### 3.13 Funciones de utilidad faltantes — G13

Ninguna de las siguientes existe hoy: `clipboard`, `sleep`, `time`, `hash`, `quote` (escape de shell), `target_os/family`, `uid/gid/user_name/group_name/host_name`, `json_encode/decode`, `image_show/precache/info`.

### 3.14 Sin estado mutable por plugin — G14

`pairee.settings` es read-only, derivado del manifest. No hay equivalente a una tabla mutable por plugin que persista entre llamadas. Un plugin que necesita un contador, una caché, o un timestamp de "última vez visto" debe re-derivar todo en cada llamada.

---

## 4. Fundamentos Arquitectónicos

Las 14 brechas caen en 6 áreas temáticas. Las áreas se construyen sobre tres fundamentos arquitectónicos compartidos que se introducen primero.

### 4.1 Fundamento F1 — Userdata tipado con metamétodos y patrón builder

**Por qué**: cada área A, B, C, D depende de ello. Los plugins deben recibir tipos Rust reales (no tablas) para poder llamar métodos, comparar por igualdad, concatenar, y convertir a string.

**Diseño**:
- Cada valor pasado a Lua que tiene más de 2 campos es un `mlua::UserData`.
- Cada tipo tiene `__tostring` (para que `tostring(x)` funcione), y la mayoría tiene `__eq` (para que `a == b` funcione) y `__concat` (para que `..` funcione).
- Cada valor colección es iterable vía `__pairs` o `__index` (para que `#files`, `for _, f in ipairs(files)` funcionen).
- Cada método builder devuelve el userdata mismo (no un nuevo valor), permitiendo encadenamiento.
- Un pequeño shim `add_cached_field` memoiza campos derivados (p. ej. `Url.name`) en el handle del userdata — la primera llamada computa, las llamadas siguientes devuelven el `mlua::Value` cacheado en el named user value del userdata.

### 4.2 Fundamento F2 — Maquinaria de contexto sync

**Por qué**: las áreas D (`cx`, `rt`, `th`, `km`) y F (bridge sync/async) dependen de ello. Los previewers, hooks, y componentes UI pluggables necesitan leer estado vivo, no un snapshot one-shot.

**Diseño**:
- Un struct `Runtime` almacenado como Lua app data: `{ blocking: bool, frames: VecDeque<RuntimeFrame>, blocks: HashMap<String, Vec<Function>> }`.
- Una macro `runtime_scope!(lua, id, block)` empuja un frame, establece `blocking=true`, ejecuta el bloque, saca el frame.
- La ejecución de plugins sync corre en el mismo hilo que el loop principal de eventos, dentro de un `runtime_scope!`. El hilo principal posee el único estado `Lua`.
- La ejecución de plugins async corre en estados Lua aislados (sin `cx`, sin `rt`, sin `th`); estos plugins usan `pairee.sync(fn)` para hacer bridge al contexto sync.
- El flag `blocking` se verifica al inicio de cada API interactiva (`pairee.which`, `pairee.input`, `pairee.confirm`) — si es `true`, la llamada lanza para prevenir re-entrada.
- `pairee.sync(fn)` y `pairee.async(fn)` son los bridges. En contexto sync, el bridge llama la función directamente. En contexto async, el bridge serializa los args, los envía al hilo principal vía un canal de callback, espera el resultado.

### 4.3 Fundamento F3 — Convención estándar de multi-retorno `(value, Error?)`

**Por qué**: cada API async en las áreas B y E depende de ello. Sin errores tipados, los plugins no pueden distinguir entre "permiso denegado" y "archivo no encontrado" y "red caída".

**Diseño**:
- Cada función async que puede fallar devuelve `(value, Error?)`. En éxito, `Error` es `nil`. En fallo, `value` es `nil` y `Error` es un userdata con campos `code` (i32 | nil) y `kind` (string | nil) más un `__tostring` que devuelve el mensaje de error del SO.
- Un helper `Err(s, ...)` en `src/plugin/runtime/presets/ya.lua` construye un `Error.custom` desde un format string.
- Compatibilidad hacia atrás: las `pairee.fs.read`/`write` existentes (que actualmente lanzan en error) mantienen su comportamiento de lanzar; solo las nuevas APIs usan la tupla.

### 4.4 Fundamento F4 — `pairee.state` mutable por plugin

**Por qué**: G14 no puede cerrarse extendiendo `pairee.settings` (read-only por diseño). Es un concepto distinto que se sienta junto a `pairee.settings` y `pairee.t()`.

**Diseño**:
- `pairee.state` es una tabla Lua por instancia de plugin, mutable, persiste entre llamadas, almacenada en `Runtime::blocks` indexada por nombre de plugin.
- El estado es de primera clase: puede pasarse a `pairee.sync(function(state) ... end)` como primer argumento.
- El estado se destruye cuando el plugin se descarga.

---

## 5. Superficie de la Nueva API por Área

Las 6 áreas se construyen sobre los 4 fundamentos anteriores. Cada área lista la nueva API, su firma Lua, su módulo Rust, y la fase de implementación.

### 5.A UserData Tipado (fundación)

**A1. Userdata `Url`**
- Constructores: `Url("path" | "sftp://host//path")`, `Url(other_url)` (clonar).
- Campos (cached): `path`, `name`, `stem`, `ext`, `urn`, `base`, `parent`, `scheme`, `domain`.
- Campos (directos): `is_regular`, `is_search`, `is_archive`, `is_absolute`, `has_root`.
- Métodos: `join(other)`, `starts_with(base)`, `ends_with(child)`, `strip_prefix(base)`, `into_search(domain)`.
- Metamétodos: `__eq`, `__tostring`, `__concat`.
- **Rust**: `src/plugin/types/url.rs`.

**A2. Userdata `Path`**
- Constructor: `Path.os("string")` (o `Path("string")`).
- Campos: `name`, `stem`, `ext`, `parent`, `is_absolute`, `has_root`.
- Métodos: `join`, `starts_with`, `ends_with`, `strip_prefix`.
- Metamétodos: `__eq`, `__tostring`, `__concat`.
- **Rust**: `src/plugin/types/path.rs`.

**A3. Userdata `Cha` (características del archivo)**
- Constructor: `Cha{...}` desde tabla.
- Campos: `mode` (u16), `is_dir`, `is_hidden`, `is_link`, `is_orphan`, `is_dummy`, `is_block`, `is_char`, `is_fifo`, `is_sock`, `is_exec`, `is_sticky`, `len`, `atime`, `btime`, `mtime`, `uid`, `gid`, `nlink`.
- Métodos: `perm()` → string (representación de permisos Unix; `nil` en Windows), `hash(long?)` → hex.
- **Rust**: `src/plugin/types/cha.rs`. Bitflags internos `ChaKind` (FOLLOW/HIDDEN/SYSTEM/DUMMY) + `ChaMode` (T_FILE/T_DIR/T_LINK/T_BLOCK/T_CHAR/T_FIFO/T_SOCK + S_SUID/S_SGID/S_STICKY + 9 bits de permiso).

**A4. Userdata `File`** (el punto de entrada principal)
- Constructor: `File{url=Url, cha=Cha}` o `File(other_file)` (clonar).
- Campos: `cha`, `url`, `link_to`, `name`, `path`, `cache`.
- Métodos: `icon()`, `size()`, `mime()`, `prefix()`, `style()`, `is_selected()`, `is_yanked()`, `found()`, `hash()`.
- `File` hace Deref a `Cha`, así que todos los campos y métodos de `Cha` son también accesibles directamente en `File`.
- **Rust**: `src/plugin/types/file.rs` con `impl Deref<Target = Cha>`.

**A5. Userdata `Error`**
- Constructores: `Error.custom("msg")`, `Error.fs({kind, code, message})`.
- Campos: `code` (i32 | nil), `kind` (string | nil).
- Metamétodos: `__tostring`, `__concat`.
- Todos los métodos `fs.*` y `Command.*` devuelven tuplas multi-valor `(value, Error?)`.
- **Rust**: `src/plugin/types/error.rs`. Añadir helper `Err(s, ...)` en `src/plugin/runtime/presets/ya.lua`.

**A6. Soporte de URL SFTP** (reusa el crate `ssh2` existente en `Cargo.toml:37,42`)
- `Url("sftp://user@host:port//path")` parsea al enum de esquema de URL.
- La nueva capa de abstracción VFS despacha a `ssh2` para metadata/read/write.
- Esta es una capacidad nueva sustancial (navegación de FS remoto) y se envía en M2.

**A7. Preview de imagen en plugins**
- `pairee.image.show(url, rect)` → reusa el crate `image` existente para decodificar, integra con la pila de adaptadores de imagen de terminal (Chafa / iTerm2 / Sixel) vía un nuevo módulo interno `pairee-tty-adapter`.
- `pairee.image.precache(src, dist)` → redimensionar + escribir a directorio cache.
- `pairee.image.info(url)` → `{w, h, format, color}`.
- **Rust**: `src/plugin/runtime/bindings/image.rs`.

**A8. `Error` tipado en todas partes**
- La convención de multi-retorno `(value, Error?)` se convierte en la convención para cada función async en `fs.*`, `Command.*`, y cualquier API async futura.
- **Compatibilidad hacia atrás**: las `pairee.fs.read`, `pairee.fs.write` existentes que devuelven strings (actualmente lanzan en error) mantienen su comportamiento de lanzar, pero las nuevas APIs usan la tupla.

**Fase**: **M2 (fundación UserData)**.

### 5.B Filesystem + Command (async, streaming)

**B1. Hacer `fs.*` async con `tokio::fs`**
- Reemplazar `std::fs::read_to_string` etc. con `tokio::fs::read_to_string` para que el hilo worker del plugin no se bloquee.
- Todos los nuevos `fs.*` devuelven `(value, Error?)`.

**B2. Añadir operaciones `fs.*` faltantes**
- `fs.mkdir(type, url)` donde `type ∈ {"dir", "dir_all"}`.
- `fs.remove(type, url)` donde `type ∈ {"file", "dir", "dir_all", "dir_clean"}`.
- `fs.rename(from, to)`.
- `fs.copy(from, to)` → bytes copiados (background, cancelable).
- `fs.read_dir(url, {glob?, limit?, resolve?})` → `File[]`.
- `fs.unique(type, url)` → `Url` único.
- `fs.cha(url, follow?)` → `Cha`.
- `fs.file(url)` → `File`.
- `fs.expand_url(value)` → `Url`.
- `fs.partitions()` → `Partition[]` (para la integración UI de ChDrive).
- `fs.calc_size(url)` → `SizeCalculator` (produce Cha mientras camina el árbol).

**B3. Builder `Command`**
- `Command("ls"):arg(...):cwd(...):env(k,v):stdin(Stdio):stdout(Stdio):stderr(Stdio):memory(max):spawn()→Child, :output()→Output, :status()→Status`.
- `Stdio ∈ {Command.NULL, Command.PIPED, Command.INHERIT}`.
- **Rust**: `src/plugin/runtime/bindings/process/command.rs`.

**B4. Userdata `Child`**
- Métodos: `:id()`, `:read(len)`, `:read_line()`, `:read_line_with({timeout})`, `:write_all(src)`, `:flush()`, `:wait()`, `:wait_with_output()`, `:try_wait()`, `:start_kill()`, `:take_stdin()`, `:take_stdout()`, `:take_stderr()`.
- `read_line` usa `tokio::select!` para competir stdout vs stderr.
- **Rust**: `src/plugin/runtime/bindings/process/child.rs`.

**B5. Userdata `Output` y `Status`**
- `Output { status: Status, stdout: string, stderr: string }`.
- `Status { success: bool, code: number? }`.

**B6. Builder `Access` y userdata `Fd`**
- `fs.access():read(true):write(true):open(url) → Fd`.
- `Fd:write_all(bytes)`, `Fd:flush()`, `Fd:read(len)`.

**Fase**: **M2 (UserData) + M3 (Async fs/Command)**.

### 5.C Widgets UI (ratatui completo)

**C1–C6. Widgets básicos como userdata con patrón builder**
- `ui.Span("text")`, `ui.Line(...)`, `ui.Text(...)`, `ui.List(...)`, `ui.Paragraph(text)`, `ui.Table(...)` — mismos nombres que hoy, pero devueltos como userdata con `:style(s)`, `:area(rect)`, `:fg(color)`, `:bg(color)`, `:bold()`, `:italic()`, `:underline()`, `:align(...)`, `:wrap(...)`, `:width()`, `:visible()`.
- `ui.Text.parse(ansi_string)` → `ui.Text` con secuencias de escape ANSI decodificadas.
- El `__call` de `ui.Span` también acepta otro `ui.Span` (clonar). El de `ui.Line` acepta string, Span, o tabla mixta.
- **Rust**: cada widget es un `mlua::UserData` en `src/plugin/runtime/bindings/ui/elements/{span,line,text,list,paragraph,table}.rs`. Los métodos builder devuelven `ud` para encadenar.

**C7. Userdata `ui.Style`**
- `:fg(color)`, `:bg(color)`, `:bold()`, `:dim()`, `:italic()`, `:underline()`, `:blink()`, `:reverse()`, `:hidden()`, `:crossed()`, `:reset()`, `:patch(other_style)`, `:raw()`.
- Color: acepta string (`"red"`, `"#ff0000"`, `"rgb(255,0,0)"`), `ui.Color(userdata)`, o `nil` (reset).
- `Style` es heredado por Span/Line/Text (llamadas a métodos encadenados).

**C8. `ui.Layout` + `ui.Constraint` + `ui.Rect`**
- `ui.Rect{x,y,w,h}`, campos: `x, y, w, h, left, right, top, bottom`, método `:pad(Pad)`.
- `ui.Layout():direction(Layout.HORIZONTAL|VERTICAL):margin(n):constraints({...}):split(rect) → Rect[]`.
- `ui.Constraint.{Min, Max, Length, Percentage, Ratio, Fill}` factories.

**C9. `ui.Pad`, `ui.Pos`, `ui.Align`, `ui.Wrap`, `ui.Edge`**
- `ui.Pad(top,right,bottom,left)` con métodos factory `Pad.left(n)`, `Pad.right(n)`, `Pad.top(n)`, `Pad.bottom(n)`, `Pad.x(n)`, `Pad.y(n)`, `Pad.xy(x,y)`.
- `ui.Pos { "top-center", x, y, w, h }` para posicionar diálogos.
- `ui.Align.LEFT|CENTER|RIGHT`.
- `ui.Wrap.NO|YES|TRIM`.
- `ui.Edge.NONE|TOP|RIGHT|BOTTOM|LEFT|ALL` (bitmask).

**C10. `ui.Border`, `ui.Bar`, `ui.Clear`, `ui.Gauge`, `ui.Fill`**
- `ui.Border(Edge):type(Border.PLAIN|ROUNDED|DOUBLE|THICK|QUADRANT_INSIDE|QUADRANT_OUTSIDE):style(s):title(Line, Edge?):merge(bool)`.
- `ui.Bar(Edge):symbol(str):style(s)`.
- `ui.Clear(Rect)`.
- `ui.Gauge():ratio(0..1)|percent(n):label(span):style(s):gauge_style(s)`.
- `ui.Fill(rect):style(s)`.

**C11. `ui.Table` y `ui.Row`, `ui.Cell`**
- `ui.Table({Row, Row, ...}):header(Row):footer(Row):widths({Constraint, ...}):spacing(n):style(s):row_style(s):col_style(s):cell_style(s):row(n?):col(n?)`.
- `ui.Row({Cell, ...}):style(s):height(n):margin_t(n):margin_b(n)`.
- `ui.Cell` es un wrapper transparente (string/Span/Line/Text).

**C12. Despacho Renderable**
- Un enum `Renderable` en Rust envuelve cada variante de widget. Los callbacks Lua que devuelven un `Renderable` (p. ej., desde un futuro patrón `c:redraw(area)`) se despachan al backend ratatui.
- **Rust**: `src/plugin/runtime/bindings/ui/renderable.rs`.

**Fase**: **M4 (Widgets UI)**.

### 5.D Contexto (`cx`), Runtime (`rt`), Theme (`th`), Keymap (`km`)

**D1. Global `cx` — solo sync**
- Establecido durante el contexto sync por el hilo principal; `nil` en contextos async.
- Árbol: `cx.active` (Tab), `cx.tabs`, `cx.tasks`, `cx.yanked`, `cx.input`, `cx.which`, `cx.layer`.
- `cx.active` es un `Tab` con: `id`, `name`, `mode` (is_select/is_unset/is_visual), `pref` (sort_by, sort_sensitive, sort_reverse, sort_dir_first, show_hidden, linemode), `current` (Folder), `parent` (Folder?), `selected` (File[] iterable), `preview` (skip, folder), `finder` (filter string).
- `Folder` tiene: `cwd` (Url), `files` (Entries), `window` (Entries), `offset`, `cursor`, `hovered` (File?).
- `Entries` tiene ventana: `#entries`, `entries[i]` (1-indexed, offset con ventana).
- `File` aquí es el userdata de §5.A4 con campos extra: `idx`, `is_hovered`, `in_current`, `in_preview`.
- **Rust**: `src/plugin/lives/`. Solo registrado en estado Lua sync.

**D2. Global `rt` — siempre disponible**
- `rt.args.entries`, `rt.args.cwd_file`, `rt.args.chooser_file`.
- `rt.term.light`, `rt.term.cell_size()` → `w, h`.
- `rt.mgr.{sort_by, sort_sensitive, sort_reverse, sort_dir_first, show_hidden, scrolloff, mouse_events}` (algunos mutables vía `ArcSwap` para actualizaciones vivas; otros read-only).
- `rt.plugin.{fetchers, spotters, preloaders, previewers}` (tablas de reglas).
- `rt.preview.{wrap, tab_size, max_width, max_height, cache_dir, image_delay, image_filter, image_quality}`.
- `rt.tasks.{file_workers, plugin_workers, fetch_workers, preload_workers, process_workers, image_alloc, image_bound, suppress_preload}`.
- `rt.open.rules`, `rt.opener`, `rt.tty:queue(...)`, `rt.tty:flush()`.
- **Rust**: `src/plugin/runtime/bindings/rt.rs`.

**D3. Global `th` — read-only vivo**
- `th.app`, `th.mgr`, `th.tabs`, `th.mode`, `th.indicator`, `th.status`, `th.which`, `th.confirm`, `th.spot`, `th.notify`, `th.pick`, `th.input`, `th.cmp`, `th.tasks`, `th.help`.
- Cada hoja es un userdata `ui.Style`.
- En contextos async/aislados, materializa un snapshot al inicio.
- **Rust**: `src/plugin/runtime/bindings/th.rs`.

**D4. Global `km` — read-only vivo**
- `km[layer_name]` → tabla de sección de keymap para esa capa.
- **Rust**: `src/plugin/runtime/bindings/km.rs`.

**D5. Global `ps` — pub/sub con bridge remoto opcional**
- Mantener `pairee.ps.{sub, pub, unsub}` para local. Añadir `pairee.ps.{sub_remote, pub_to, unsub_remote}` solo si/cuando se implemente un bridge entre instancias (fuera de alcance de M0–M4).

**D6. Maquinaria de contexto sync**
- Introducir un `Runtime { blocking: bool, frames: VecDeque<RuntimeFrame>, blocks: HashMap<String, Vec<Function>> }` en datos de la app.
- Macro `runtime_scope!(lua, id, block)` establece `blocking=true`, empuja frame, ejecuta, saca.
- `pairee.sync(fn)` y `pairee.async(fn)` siguiendo el patrón de contexto sync.
- Este es el cambio individual más grande de la propuesta; se gatea sobre el refactor de userdata.

**D7. `pairee.state` mutable por plugin**
- Distinto de `pairee.settings` (read-only, desde manifest) y `pairee.t()` (i18n).
- Vida: por instancia de plugin, persiste entre llamadas.
- Implementado como una tabla Lua por plugin almacenada en el mapa `Runtime::blocks`.
- **Fase**: **M3 (Async fs/Command + contexto sync)**.

**Fase**: **M3**.

### 5.E Diálogos, `pairee.emit`, utils

**E1. `pairee.input` y `pairee.confirm` reales**
- Arreglar el stub en `src/plugin/manager.rs:207-223`.
- Nuevo `pairee.input({pos, title, value, obscure, realtime, debounce})` que devuelve `(value, event)` o `Recv` para realtime.
- Nuevo `pairee.confirm({pos, title, body})` que devuelve boolean.
- Añadir nuevas variantes `PluginRequest::InputDialog` y `ConfirmDialog` que abran los popups TUI existentes (`src/ui/popup.rs`).

**E2. `pairee.which({cands, silent})`**
- Nueva API de prompt de teclas.
- Devuelve índice 1-based del candidato seleccionado, o `nil`.
- **Rust**: nueva variante `PluginRequest::WhichPrompt` enrutada a un nuevo popup `which`.

**E3. `pairee.emit(action, args)`**
- El despacho de acción general: cualquier `Action` de `src/keybindings/actions.rs` es llamable.
- `pairee.emit("cd", {"/some/path"})` → dispara la acción `Cd`.
- `pairee.emit("select", {url, state=true})` → dispara selección.
- `pairee.emit("reveal", {url})` → revela en el panel opuesto.
- **Rust**: nueva variante `PluginRequest::EmitAction { name: String, args: serde_json::Value }`; el loop principal busca la acción en el resolver de keybindings y la ejecuta.
- **Fase**: **M0 (scaffolding)** — cambio pequeño, alto apalancamiento.

**E4. `pairee.file_cache({file, skip})`**
- Devuelve una URL de caché (basada en hash) para previewers. Previene cacheo recursivo del archivo de caché.
- **Rust**: nuevo `PluginRequest::FileCache`.

**E5. `pairee.preview_code({area, file, mime, skip})` y `pairee.preview_widget(opts, widget)`**
- `preview_code` integra el `pulldown-cmark` existente y añade highlighting de sintaxis (no hay resaltador hoy; añadir `syntect` o usar shell-out).
- `preview_widget` escribe un widget directamente en el panel de preview (extiende `UpdatePluginWidget` para aceptar cualquier `Renderable`).

**E6. `pairee.notify({title, content, timeout, level})`**
- Se alinea con la firma estándar. Extender el `Notify` request existente para tomar un payload estructurado.

**E7. `pairee.clipboard(text?)`**
- Get/set del portapapeles del sistema.
- **Sandbox**: bloquear `get` en secure mode; permitir `set` solo dentro del workspace.

**E8. `pairee.quote(str, unix?)`**
- Escapa para shell un string. Usa el escape OS-específico existente en `src/terminal/`.

**E9. `pairee.{sleep, time, hash, target_os, target_family, json_encode, json_decode, percent_encode, percent_decode}`**
- `sleep` es async (usa `tokio::time::sleep`); el resto son sync.
- `hash` usa XxHash3-128 para hashing estable y rápido.
- `json_encode`/`decode` usan el `serde_json` existente.

**E10. `pairee.{uid, gid, user_name, group_name, host_name}` (solo Unix)**
- En Windows, devolver `nil` (no error) para que los plugins puedan escribir código portable.
- Usar el crate `uzers` (o `getpwuid`/`getgrgid` vía `libc`).

**Fase**: **M1 (scaffolding + emit + arreglos de diálogos) + M4 (utils)**.

### 5.F Sync/Async, anotaciones `@sync`/`@since`, preloader/seek

**F1. Contextos sync vs async**
- Introducir un struct `Runtime`.
- Añadir macro `runtime_scope!` para establecer `blocking=true` durante la ejecución de plugins sync.
- Los plugins async obtienen estados Lua slim aislados sin los globales vivos y con `th` materializado.

**F2. Parseo de anotaciones `@sync` / `@since`**
- Parsear `--- @sync entry` y `--- @sync peek` de los comentarios de `main.lua` en tiempo de carga.
- Parsear `--- @since 0.7.0` y verificar en tiempo de carga contra `CARGO_PKG_VERSION`.
- Almacenar en el struct info del plugin en el registro.
- **Rust**: `src/plugin/loader.rs` extiende `load_plugin` para leer líneas de anotación de `main_content` antes de la evaluación.
- **Fase**: **M3**.

**F3. Enrutamiento de `preload()` y `seek()`**
- Añadir `PluginTaskRequest::Preload { job, reply_tx }` y `PluginTaskRequest::Seek { job, reply_tx }` a `src/plugin/registry.rs:17-29`.
- `Preload` devuelve `(complete: bool, err: Option<String>)` (patrón estándar).
- `Seek` actualiza sincrónicamente `skip` y dispara otro `Peek`.
- **Fase**: **M3**.

**F4. `pairee.sync(fn)` y `pairee.async(fn)`**
- `pairee.sync(fn)` — crea un bloque sync, llamable desde contexto async para hacer bridge al estado sync.
- `pairee.async(fn)` — lanza una función en el async local set del hilo actual.
- Ambos siguen el mismo patrón: path sync es directo, path async usa un canal de callback.
- **Fase**: **M3**.

---

## 6. Mapeo de Secure Mode para las Nuevas APIs

Cuando Secure Mode está activo (`pairee.toml` `[settings] secure_mode = true`), cada nueva API debe clasificarse:

| Nueva API | Riesgo en Secure Mode | Acción recomendada |
|---|---|---|
| `pairee.clipboard` get | **Alto** (vector de exfiltración de datos) | Bloquear (devolver nil + warn) |
| `pairee.clipboard` set | Medio (podría filtrarse vía paste) | Permitir solo si el valor está en el workspace |
| Builder `Command` con stdio `INHERIT` | Medio (podría evadir blacklist vía proceso shell hijo) | Mantener chequeo `is_command_safe`; también bloquear stdio `INHERIT` en secure mode |
| `Command` con stdio `PIPED` | Bajo (controlado por el plugin) | Permitir |
| `pairee.image.show/precache/info` | Bajo (solo lee archivos locales) | Permitir |
| `fs.read_dir` con `resolve: true` | Medio (podría seguir symlinks fuera del sandbox) | Forzar `validate_path` sobre el resultado |
| `fs.create`, `fs.remove`, `fs.rename` | Alto (podría escapar del workspace) | Restringir a workspace + config + cache dirs (igual que el `validate_path` actual) |
| `fs.unique` | Bajo | Permitir |
| `pairee.uid/gid/user_name/group_name/host_name` | Bajo (divulgación de info) | Permitir |
| `pairee.target_os/family`, `pairee.time` | Bajo | Permitir |
| `pairee.hash` | Bajo | Permitir |
| `pairee.quote` | Bajo (operación de string) | Permitir |
| `pairee.json_encode/decode`, `percent_encode/decode` | Bajo | Permitir |
| `pairee.emit(action)` | Bajo (las acciones son validadas por el resolver) | Permitir (pero bloquear emit a acciones peligrosas como `delete` en secure mode) |
| `pairee.which`, `pairee.input`, `pairee.confirm` | Bajo (UI local) | Permitir |
| `pairee.preview_code/widget` | Bajo | Permitir |
| `pairee.file_cache` | Bajo | Permitir |
| `cx`, `rt`, `th`, `km` | Bajo (estado read-only) | Permitir (read-only) |

---

## 7. Migración y Compatibilidad Hacia Atrás

Pairee ya tiene una API pública documentada (`pairee.app.*`, `pairee.fs.*`, `pairee.ui.*`, `pairee.ps.*`). Cambiar `FileEntrySnapshot` a un userdata `File`, renombrar `pairee.fs.list` a `pairee.fs.read_dir`, etc. romperá los plugins existentes.

**Migración recomendada en dos fases**:

1. **M0 (Scaffolding, API paralela)**: introducir todas las nuevas APIs bajo nombres nuevos (`pairee.file`, `pairee.fs.read_dir`, `pairee.cha`, `Command`, etc.). Mantener la API antigua basada en tablas `pairee.app.*`, `pairee.fs.*` funcionando como wrappers delgados. Emitir un `log::warn!` de deprecación cuando se invoque la API antigua.

2. **M5 (Limpieza, API única)**: después de un ciclo de release con ambas APIs en producción, eliminar la API antigua en un salto de versión mayor. Actualizar `docs/plugin-dev-guide.md` para referenciar solo la nueva API. Actualizar la rama `plugin-template` en sincronía.

3. **Migración de manifest**: bumpear `min_pairee` en cualquier plugin empaquetado. No se requiere cambio de schema de manifest; los manifests existentes siguen funcionando porque los nuevos campos son opcionales.

4. **Documentación**: actualizar `docs/plugin-dev-guide.md` y `docs/technical/plugin-system-design.md` para reflejar la nueva API. El nuevo `plugin-roadmap.md` (este documento) es la referencia de migración. La versión en español (`plugin-roadmap-es.md`) también se publica para paridad.

---

## 8. Plugins de Aceptación

Para validar que la superficie de runtime propuesta es suficiente, el trabajo M0–M4 debería incluir el porte de al menos tres plugins de aceptación. Las implementaciones de referencia en `example/reference/` (un sistema de plugins de código abierto) proveen los patrones fuente.

1. **`fzf.pairee`** — navegación difusa de archivos. Ejercita el builder `Command` (con stdio `PIPED` e `INHERIT`), `Child:write_all` / `wait_with_output`, `ui.hide` / `Permit`, `pairee.cx.active.selected`, `pairee.cx.active.current.cwd`, `pairee.emit("cd" | "reveal" | "toggle_all")`. Objetivo: < 100 líneas de Lua.

2. **`zoxide.pairee`** — navegación de historial. Ejercita `pairee.ps.sub("cd", ...)`, `pairee.async`, `pairee.target_os/family`, `pairee.emit("cd")`, `ui.hide`, `Command:env`. Objetivo: < 150 líneas de Lua.

3. **`code-preview.pairee`** — preview de código con highlighting. Ejercita `pairee.preview_code` (o shell-out a `pygmentize` / `bat`), `pairee.ui.Line` / `ui.Text` / `ui.Span` con estilos, acceso a `pairee.th`.

Un porte exitoso de los tres confirma que la nueva API es completa y ergonómica. Un fallo en cualquiera señala una brecha a cerrar antes de la limpieza de M5.

---

## 9. Roadmap de Implementación

Esfuerzo total estimado: ~10–14 semanas para un único desarrollador Rust+Lua con experiencia. Las fases pueden correr en paralelo donde las dependencias lo permitan.

### M0 — Scaffolding (1 semana)

- Añadir nuevas variantes de `PluginRequest`: `EmitAction`, `FileCache`, `InputDialog` (reemplazando el stub), `ConfirmDialog` (reemplazando el stub), `WhichPrompt`.
- Implementar los nuevos dispatchers de request en `process_plugin_requests` (`src/plugin/manager.rs:164-287`).
- Añadir un `log::warn!` de deprecación en los paths antiguos de `Confirm` y `Input` (mantener el stub pero loguear fuerte).
- Cablear `pairee.emit("cd" | "set_focus")` como prueba de concepto para `EmitAction`.
- Añadir `pairee.file_cache`, `pairee.notify` (extendido), `pairee.target_os/family`, `pairee.time`, `pairee.hash` (shell-out a un pequeño helper Rust por ahora — XxHash completo después).
- **Termina cuando**: un plugin puede llamar `pairee.emit("cd", {"/tmp"})` y el panel navega.

### M1 — Utils Esenciales + Diálogos Reales (1.5 semanas)

- Implementar `pairee.input` real (con realtime, debounce, obscure) y `pairee.confirm` (con pos/title/body) enrutando a los popups TUI.
- Añadir `pairee.which`.
- Añadir `pairee.quote`, `pairee.percent_encode/decode`, `pairee.json_encode/decode`, `pairee.sleep`.
- Añadir `pairee.uid/gid/user_name/group_name/host_name` (solo Unix; `nil` en Windows).
- Añadir `pairee.clipboard` con gating de secure mode.
- **Termina cuando**: un plugin puede llamar `pairee.input({title="Name"})` y el usuario ve un diálogo de input real con el valor devuelto.

### M2 — Fundación de UserData Tipado (3 semanas)

- Añadir los userdata `Url`, `Path`, `Cha`, `File`, `Error` (según §5.A1–A5).
- Añadir shim `add_cached_field` para memoizar campos derivados.
- Añadir el proxy `Composer` para resolución lazy del namespace.
- Reemplazar `FileEntrySnapshot` con el nuevo userdata `File`, manteniendo el struct antiguo como tipo interno deprecado.
- Actualizar la construcción del job de `peek` en `src/plugin/registry.rs:104-149` para pasar un userdata `File` real con `cha` y `mime`.
- Añadir `pairee.image.show/precache/info` reusando el crate `image` existente.
- **Termina cuando**: un plugin puede llamar `entry.cha:perm()` y obtener un string de permisos Unix; un plugin puede llamar `pairee.image.show(url, rect)` y ver la imagen en el panel de preview.

### M3 — fs Async + Command + Contexto Sync (3 semanas)

- Añadir el struct `Runtime` y la macro `runtime_scope!` (`src/plugin/runtime/runtime.rs` + `src/plugin/macros.rs`).
- Implementar el dispatch de plugins sync vs async.
- Parsear las anotaciones `--- @sync entry` y `--- @sync peek` en tiempo de carga.
- Añadir el enrutamiento de `preload()` y `seek()`.
- Migrar `pairee.fs.read/write/exists/stat/list` a `tokio::fs` (no bloqueante).
- Añadir las nuevas operaciones `fs.*` según §5.B2.
- Añadir el builder `Command`, `Child`, `Output`, `Status`, `Access`, `Fd` según §5.B3–B6.
- Añadir `pairee.state` según §4.F4.
- Portar `fzf.pairee` y `zoxide.pairee` como tests de aceptación.
- **Termina cuando**: un plugin puede escribir `Command("fzf"):arg("-m"):stdin(PIPED):stdout(PIPED):spawn()` y streamear input/output; un plugin puede llamar `pairee.cx.active.current.hovered` y leer estado vivo.

### M4 — Widgets UI + cx/rt/th/km (3 semanas)

- Añadir los 24 widgets UI según §5.C1–C11 como userdata.
- Añadir el enum `Renderable` y el despacho a ratatui.
- Añadir `cx`, `rt`, `th`, `km` según §5.D1–D4.
- Añadir `pairee.preview_code/widget`.
- Portar `code-preview.pairee` como test de aceptación.
- **Termina cuando**: un plugin puede construir `ui.Line("hello"):fg("red"):bold()` y verlo renderizado en el panel de preview en texto rojo y negrita.

### M5 — Limpieza (1 semana)

- Eliminar las APIs deprecadas de M0 (o mantener un release más).
- Actualizar `docs/plugin-dev-guide.md` y `docs/technical/plugin-system-design.md` para reflejar la nueva API.
- Actualizar la rama `plugin-template` en sincronía.
- Añadir warnings de deprecación a cualquier llamador restante de la API antigua.
- Ejecutar un "sprint de migración": portar todos los plugins publicados de Pairee (git.pairee, fzf.pairee, etc.) a la nueva API.
- Ejecutar un `pairee developer validate` en todo el registro para asegurar que todos los plugins empaquetados cumplen.
- **Termina cuando**: un `cargo build` y `cargo test` limpios; no quedan llamadas a `pairee.fs.list` en los plugins empaquetados.

---

## Apéndice A — Mapeo de Pairee Actual → Propuesto

Referencia cruzada para el equipo de implementación.

| Pairee (actual) | Pairee (propuesto) | Notas |
|---|---|---|
| `pairee.app.cwd()` | `pairee.app.cwd()` | Igual |
| `pairee.app.cd(path)` | `pairee.app.cd(path)` | Mantenido por compat; `pairee.emit("cd", {path})` es la forma unificada |
| `pairee.app.focus()` | `pairee.app.focus()` | Igual |
| `pairee.app.set_focus(side)` | `pairee.app.set_focus(side)` | Mantenido; `pairee.emit("focus", {side})` es la forma unificada |
| `pairee.app.notify(title, msg, level)` | `pairee.notify({title, content, timeout, level})` | Nueva forma estructurada |
| `pairee.app.confirm(title, msg)` (stub) | `pairee.confirm({pos, title, body})` | Ahora real |
| `pairee.app.input(title, default)` (stub) | `pairee.input({pos, title, value, obscure, realtime, debounce})` | Ahora real |
| `pairee.app.hovered()` | `pairee.app.hovered()` → devuelve userdata `File` | Más rico |
| `pairee.fs.read(path)` | `pairee.fs.read(url)` (async) | Ahora no bloqueante |
| `pairee.fs.write(path, data)` | `pairee.fs.write(url, data)` (async) | Ahora no bloqueante |
| `pairee.fs.exists(path)` | `pairee.fs.cha(url)` / `pairee.fs.exists(url)` | Nuevo userdata `cha` |
| `pairee.fs.stat(path)` | `pairee.fs.cha(url)` | Nuevo userdata `cha` |
| `pairee.fs.list(path)` | `pairee.fs.read_dir(url, opts)` | Ahora async + opciones |
| `pairee.fs.spawn(cmd, args)` | `Command("..."):arg{...}:cwd():env():stdin():stdout():stderr():memory():spawn()` | Nuevo builder |
| `pairee.fs.spawn_copy_task(from, to)` | `pairee.fs.copy(from, to)` + `pairee.emit("tasks:update_succeed")` | Nueva forma |
| `pairee.ui.Paragraph(text)` | `pairee.ui.Paragraph(text)` | Mantenido; nuevo `ui.Text` preferido |
| `pairee.ui.Gauge(ratio, label)` | `pairee.ui.Gauge():ratio(r):label(span)` | Builder |
| `pairee.ui.List(items)` | `pairee.ui.List({...})` | Builder |
| `pairee.ui.Table(headers, rows)` | `pairee.ui.Table({Row, ...})` | Builder |
| `pairee.ui.Span(text, style)` | `pairee.ui.Span(text):style(s)` | Builder |
| `pairee.ui.Line(spans)` | `pairee.ui.Line({Span, ...})` | Builder |
| (n/a) | `pairee.ui.Style():fg("red"):bold()` | Nuevo |
| (n/a) | `pairee.ui.Layout():direction(H):constraints({...}):split(rect)` | Nuevo |
| (n/a) | `pairee.ui.{Rect, Pad, Pos, Border, Bar, Clear, Fill, Align, Wrap, Edge, Constraint, Color}` | Nuevo |
| `pairee.ps.sub` | `pairee.ps.sub` | Mantenido |
| `pairee.ps.pub` | `pairee.ps.pub` | Mantenido |
| `pairee.ps.unsub` | `pairee.ps.unsub` | Mantenido |
| (n/a) | `pairee.ps.pub_to` / `pairee.ps.sub_remote` / `pairee.ps.unsub_remote` | Nuevo, opcional entre instancias |
| `pairee.log.info/warn/error/debug` | `pairee.log.*` (o `pairee.dbg/err`) | Mantenido |
| `pairee.sync(fn)` | `pairee.sync(fn)` (implementación completa) | Bridge sync/async completo |
| `pairee.settings.*` | `pairee.settings.*` (read-only, del manifest) | Mantenido |
| (n/a) | `pairee.state` (mutable por plugin) | Nuevo |
| `pairee.t(key, vars)` | `pairee.t(key, vars)` | Mantenido |
| (n/a) | `pairee.emit(action, args)` | Nuevo (M0) |
| (n/a) | `pairee.exec(action, args)` | Nuevo (M0) |
| (n/a) | `pairee.file_cache(opts)` | Nuevo (M0) |
| (n/a) | `pairee.preview_code(opts)` | Nuevo (M4) |
| (n/a) | `pairee.preview_widget(opts, widget)` | Nuevo (M4) |
| (n/a) | `pairee.which(opts)` | Nuevo (M1) |
| (n/a) | `pairee.image.{show,precache,info}` | Nuevo (M2) |
| (n/a) | `pairee.clipboard(text?)` | Nuevo (M1) |
| (n/a) | `pairee.quote(str, unix?)` | Nuevo (M1) |
| (n/a) | `pairee.{sleep, time, hash, target_os, target_family, json_encode, json_decode, percent_encode, percent_decode}` | Nuevo (M1) |
| (n/a) | `pairee.{uid, gid, user_name, group_name, host_name}` (solo Unix) | Nuevo (M1) |
| (n/a) | `cx`, `rt`, `th`, `km` (solo contexto sync) | Nuevo (M3/M4) |
| (n/a) | `pairee.image.show` | Nuevo (M2) |

---

## Apéndice B — Archivos de Pairee Afectados

Una lista consolidada de cada archivo de Pairee que necesita modificación a lo largo del plan M0–M5. Los conteos son aproximados.

### Archivos nuevos (por fase)

**M0** (4 nuevos):
- `src/plugin/runtime/bindings/emit.rs`
- `src/plugin/runtime/bindings/utils_basic.rs` (file_cache, target_os, time, hash)
- `src/plugin/runtime/bindings/notify_ext.rs`
- `src/plugin/runtime/bindings/which.rs` (stub — impl completa en M1)

**M1** (3 nuevos):
- `src/plugin/runtime/bindings/dialogs.rs` (input, confirm)
- `src/plugin/runtime/bindings/clipboard.rs`
- `src/plugin/runtime/bindings/utils_ext.rs` (quote, sleep, percent, json, uid/gid, etc.)

**M2** (8 nuevos):
- `src/plugin/types/mod.rs`
- `src/plugin/types/url.rs`
- `src/plugin/types/path.rs`
- `src/plugin/types/cha.rs`
- `src/plugin/types/file.rs`
- `src/plugin/types/error.rs`
- `src/plugin/runtime/bindings/image.rs`
- `src/plugin/runtime/bindings/traits.rs` (add_cached_field, Composer)

**M3** (5 nuevos):
- `src/plugin/runtime/runtime.rs` (struct `Runtime`)
- `src/plugin/macros.rs` (runtime_scope!)
- `src/plugin/runtime/bindings/process/mod.rs`
- `src/plugin/runtime/bindings/process/command.rs`
- `src/plugin/runtime/bindings/process/child.rs` (con `access.rs` y `fd.rs`)

**M4** (10 nuevos):
- `src/plugin/runtime/bindings/ui/elements/{span,line,text,list,paragraph,table}.rs`
- `src/plugin/runtime/bindings/ui/style.rs`
- `src/plugin/runtime/bindings/ui/layout.rs` (con constraint, rect, pad, pos)
- `src/plugin/runtime/bindings/ui/borders.rs` (border, bar, clear, gauge, fill, align, wrap, edge, color)
- `src/plugin/runtime/bindings/ui/renderable.rs`
- `src/plugin/runtime/bindings/cx.rs`
- `src/plugin/runtime/bindings/rt.rs`
- `src/plugin/runtime/bindings/th.rs`
- `src/plugin/runtime/bindings/km.rs`
- `src/plugin/runtime/bindings/preview.rs` (preview_code, preview_widget)

### Archivos modificados

- `src/plugin/manager.rs` — añadir 5 nuevas variantes `PluginRequest`; añadir dispatchers en `process_plugin_requests`; arreglar stubs de diálogos.
- `src/plugin/loader.rs` — parsear anotaciones `@sync`/`@since`; bumpear chequeos de versión.
- `src/plugin/sandbox.rs` — extender `is_command_safe` y `validate_path` para las nuevas APIs.
- `src/plugin/registry.rs` — añadir variantes `Preload`/`Seek`; reemplazar tabla con userdata `File` en `Peek`.
- `src/plugin/runtime/standard.rs` — registrar nuevos globals (`pairee.state`, `pairee.image.*`, `pairee.which`, etc.).
- `src/plugin/runtime/bindings/app.rs` — reimplementar `cd`/`set_focus` como wrappers delgados sobre `EmitAction`; eliminar stubs de `confirm`/`input` (movidos a dialogs.rs).
- `src/plugin/runtime/bindings/fs.rs` — migrar a `tokio::fs`; añadir nuevas operaciones; añadir retornos `(value, Error?)`.
- `src/plugin/runtime/bindings/ui.rs` — deprecarl constructores de tablas planas; enrutar a los nuevos módulos de widgets.
- `src/plugin/runtime/bindings/sync.rs` — implementación completa del bridge (path sync/async dual).
- `src/plugin/hooks.rs` — enriquecer la superficie de eventos.
- `src/app/state/types.rs` — extender `PopupType` con `WhichPrompt` y las nuevas variantes de diálogo.
- `src/ui/popup.rs` (y `src/ui/popup/*.rs`) — añadir popup `which`, popups reales de input/confirm.
- `src/keybindings/resolver.rs` — aceptar y despachar requests `EmitAction` de plugins.
- `docs/plugin-dev-guide.md` y `docs/plugin-dev-guide-es.md` — reescribir la sección de superficie de API.
- `docs/technical/plugin-system-design.md` y `docs/technical/plugin-system-design-es.md` — actualizar al nuevo modelo.
- La rama git huérfana `plugin-template` — actualizar el boilerplate a la nueva API.

### Archivos de plugins de aceptación de referencia (nuevos en M3/M4)

- `plugins_dev_dir/fzf.pairee/manifest.toml`, `main.lua`, `lang/en.toml`, `help/en.md`
- `plugins_dev_dir/zoxide.pairee/manifest.toml`, `main.lua`, `lang/en.toml`, `help/en.md`
- `plugins_dev_dir/code-preview.pairee/manifest.toml`, `main.lua`, `lang/en.toml`, `help/en.md`

---

## Apéndice C — Material de Referencia

Un gestor de archivos TUI de terceros con un sistema de plugins Lua maduro fue usado para validar el diseño y obtener patrones concretos de las características más avanzadas (userdata tipado, bridge sync/async, builder de widgets, streaming de comandos, capa VFS). Está vendorizado en `example/reference/` (gitignored) para referencia local y no es parte del release de Pairee. Ningún nombre, fuente, o atribución se expone en la API pública de Pairee ni en este documento.

El sistema de plugins de Pairee es, y seguirá siendo, un diseño original. Las fortalezas existentes de Pairee — el tooling de desarrollo (CLI `pairee developer`, asistente TUI, rama `plugin-template`), el `settings_schema` del manifest renderizado en la TUI, la i18n por plugin con fallback, la integración de ayuda F1, el Secure Mode estricto, y el registro con verificación SHA-256 — no derivan de ningún sistema de terceros y son la fundación sobre la cual se construirá la nueva superficie de runtime.

---

*Preparado por los mantenedores de Pairee como documento de diseño interno. Todas las referencias `file:line` son precisas a la fecha de Pairee 0.6.1 (rama `main` actual). No se referencian nombres externos en este documento ni en la API propuesta de Pairee.*
