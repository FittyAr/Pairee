# Guía para Desarrolladores de Complementos de Pairee

> **Esta guía explica cómo escribir, probar y enviar un complemento Lua para Pairee.**

---

## Tabla de Contenidos

1. [Descripción General](#1-descripción-general)
2. [Tipos de Complemento](#2-tipos-de-complemento)
3. [Tu Primer Complemento — Hola Mundo](#3-tu-primer-complemento--hola-mundo)
4. [Estructura de Archivos del Complemento](#4-estructura-de-archivos-del-complemento)
5. [Resumen de la API](#5-resumen-de-la-api)
6. [Comandos y atajos de teclado](#6-comandos-y-atajos-de-teclado)
7. [Escribir un Complemento Previewer](#7-escribir-un-complemento-previewer)
8. [Escribir un Complemento Hook](#8-escribir-un-complemento-hook)
9. [Escribir un Complemento Comando](#9-escribir-un-complemento-comando)
10. [Sincronización de Estado con `pairee.sync`](#10-sincronización-de-estado-con-pairesync)
11. [Eventos Pub/Sub con `pairee.ps`](#11-eventos-pubsub-con-pairees)
12. [Depuración y Control de Errores](#12-depuración-y-control-de-errores)
13. [Modo Confiable y Protección del Modo Seguro](#13-modo-confiable-y-protección-del-modo-seguro)
14. [Probar tu Complemento Localmente](#14-probar-tu-complemento-localmente)
15. [Herramientas del Modo Desarrollador y TUI de Envío de PR](#15-herramientas-del-modo-desarrollador-y-tui-de-envío-de-pr)
16. [Flujo de Envío al Registro](#16-flujo-de-envío-al-registro)
17. [Referencia del Manifiesto](#17-referencia-del-manifiesto)
18. [Mejores Prácticas y Convenciones](#18-mejores-prácticas-y-convenciones)

---

## 1. Descripción General

Los complementos de Pairee son **módulos Lua** que extienden el gestor de archivos con:
- **Visores de archivos (previewers)** — renderizan contenido en el panel de vista previa para tipos de archivo específicos.
- **Hooks de ciclo de vida** — reaccionan a eventos de navegación (cambio de directorio, pasar el cursor sobre un archivo, pulsaciones de tecla).
- **Comandos funcionales** — ejecutan acciones invocadas mediante atajos de teclado o el menú de usuario.

Los complementos se almacenan en tu carpeta de configuración y se configuran en `config.toml`. Una vez cargados, se ejecutan de manera asíncrona en hilos de segundo plano sin bloquear el renderizado de la interfaz de terminal.

---

## 2. Tipos de Complemento

| Tipo | Métodos Lua | Invocado Cuando |
|------|-------------|-----------------|
| **Previewer** | `peek(job)`, `seek(job)` | La Vista Rápida muestra un archivo (`peek`); RePág / AvPág sobre la vista del complemento (`seek`) |
| **Preloader** | `preload(job)` | La Vista Rápida muestra un archivo, y para los tres archivos siguientes del panel |
| **Hook** | `setup(opts)`, suscripciones via `pairee.ps.sub` | Se dispara un evento de ciclo de vida (p.ej. `on_cd`) |
| **Comando** | `entry(args)` | Un atajo de teclado o entrada de menú llama al complemento por nombre |
| **Mixto** | Combinación de los anteriores | Múltiples roles |

---

## 3. Tu Primer Complemento — Hola Mundo

Crea la carpeta `~/.config/pairee/plugins/hello.pairee/`. Pairee solo carga
carpetas llamadas `<nombre>.pairee`, y el `name` del manifiesto debe ser el
nombre de la carpeta sin el sufijo (`hello`); solo se admite `[A-Za-z0-9_-]`.

Crea el archivo `~/.config/pairee/plugins/hello.pairee/main.lua`:

```lua
local M = {}

function M:entry(args)
    pairee.app.notify("¡Hola!", "Este es mi primer complemento de Pairee.", "info")
end

return M
```

Pairee llama a las funciones del complemento como métodos, con la tabla del
complemento primero, así que se declaran con `:` (`function M:entry(args)`);
con `.` el primer parámetro recibe la tabla en lugar de `args`.

Crea el archivo de manifiesto `~/.config/pairee/plugins/hello.pairee/manifest.toml`:

```toml
name = "hello"
version = "1.0.0"
description = "Complemento simple de prueba Hola Mundo"
author = "tu-usuario"
license = "MIT"
type = "command"
min_pairee = "0.11.0"  # semver; sirven pre-releases como "0.10.0-beta.1", un valor ilegible impide la carga

[[commands]]
id    = "hello"
title = "Decir hola"
```

Regístralo en `config.toml`:

```toml
[plugins.hello]
name    = "hello"
trusted = false
```

Lanza Pairee y ejecuta **Decir hola** desde la paleta de comandos (o asigna `plugin.hello.hello` a una tecla, ver la sección 6). Aparecerá una notificación.

---

## 4. Estructura de Archivos del Complemento

Un complemento es una carpeta que contiene un script Lua principal, un manifiesto y submódulos o recursos opcionales:

```text
~/.config/pairee/plugins/
└── my-plugin.pairee/       # <nombre>.pairee; name del manifiesto = "my-plugin"
    ├── main.lua              # Requerido — punto de entrada
    ├── manifest.toml         # Requerido — metadatos y atajos de teclado
    ├── utils.lua             # Submódulo opcional
    └── lang/                 # Archivos de localización opcionales
        ├── en.toml
        └── es.toml
```

`main.lua` **debe** devolver una tabla Lua `M` que contenga las funciones de ciclo de vida/activación.

---

## 5. Resumen de la API

### `pairee.app` — Control de la Aplicación

```lua
pairee._lua_api_version                 -- "1.3.0" (semver de la superficie Lua)
pairee.app.cwd()                        -- string: directorio actual
pairee.app.cd(path)                     -- navegar a path
pairee.app.focus()                      -- "left" | "right"
pairee.app.set_focus(side)              -- cambiar panel
pairee.app.notify(title, msg, level)   -- mostrar popup ("info","warn","error")
pairee.app.confirm(title, msg)          -- boolean (deprecado; abre TUI real)
pairee.app.input(title, default)        -- string (deprecado; abre TUI real)
pairee.app.hovered()                    -- snapshot tabla del archivo resaltado
```

### `pairee.confirm` / `pairee.input` / `pairee.which` — Diálogos interactivos

```lua
pairee.confirm({ pos, title, body })    -- boolean (S/Enter o N/Esc)
pairee.input({ pos, title, value, obscure, realtime, debounce })
                                        -- { value, event } o nil
pairee.which({ cands, silent })         -- índice 1-based o nil
```

### `pairee.cx` — Contexto del panel (dentro de `pairee.sync`)

```lua
pairee.sync(function()
    local cwd = pairee.cx.active.cwd
    local hovered = pairee.cx.active.hovered  -- userdata File o nil
    if hovered then
        print(hovered.name, hovered.size, hovered.mime, hovered.is_hidden)
    end
    local selected = pairee.cx.active.selected -- File[]
    local url = pairee.cx.active.url            -- userdata Url de la carpeta
end)
```

Campos de `File`: `name`, `path`, `url`, `size`, `is_dir`, `is_symlink`,
`mime`, `mtime`, `is_hidden`, `is_exec`, `cha`. `File.url` es un string simple
(la URL remota en paneles remotos, si no la ruta); envuélvelo con
`pairee.Url(file.url)` cuando necesites un `Url`. El `url` de carpeta de
`cx.active`, `cx.left`, `cx.right` y de las entradas de `cx.tabs` ya es un
userdata `Url`.

### `pairee.fs` — Sistema de Archivos y Procesos

```lua
pairee.fs.read(path)                    -- string: contenido del archivo
pairee.fs.write(path, data)            -- escribe datos en archivo
pairee.fs.exists(path)                 -- boolean
pairee.fs.stat(path)                   -- userdata File (o nil)
pairee.fs.list(path)                   -- File[]: listado del directorio
pairee.fs.data_dir()                   -- string: directorio privado escribible del complemento
pairee.fs.read_dir(path)               -- File[] (igual que list)
pairee.fs.file(path)                   -- userdata File
pairee.fs.mkdir("dir"|"dir_all", path)
pairee.fs.remove("file"|"dir"|"dir_all"|"dir_clean", path)
pairee.fs.rename(from, to)
pairee.fs.copy(from, to)               -- bytes copiados
pairee.fs.spawn(cmd, args)             -- Output: {stdout, stderr, status}
pairee.fs.spawn_copy_task(from, to)    -- copia en segundo plano con barra de progreso
```

### `pairee.Command` — builder de procesos (plugins trusted)

```lua
local out = pairee.Command("rg")
    :arg("-n")
    :arg(pattern)
    :cwd(pairee.cx.active.cwd)
    :stdout(pairee.Command.PIPED)
    :stderr(pairee.Command.PIPED)
    :output()                       -- Output { stdout, stderr, status }

local child = pairee.Command("cat")
    :stdin(pairee.Command.PIPED)
    :stdout(pairee.Command.PIPED)
    :spawn()
child:write_all("hello\n")
child:close_stdin()
local streamed = child:wait_with_output()
```

En Modo Seguro (13.3) `:env(nombre, valor)` solo acepta variables de idioma,
terminal y color (`LANG`, `LANGUAGE`, `LC_*`, `TERM`, `COLORTERM`, `NO_COLOR`,
`CLICOLOR`, `CLICOLOR_FORCE`, `FORCE_COLOR`, `TZ`, `COLUMNS`, `LINES`);
cualquier otro nombre provoca una violación de seguridad. `:cwd(ruta)` debe
indicar una carpeta existente dentro de la jaula de lectura de `pairee.fs` del
complemento.

### `pairee.ui` — Widgets

Los widgets son userdata que se construyen encadenando llamadas; `peek`
devuelve uno (ver sección 7). Todo widget y todo texto acepta los métodos de
estilo `:style(s)`, `:fg(color)`, `:bg(color)`, `:bold()`, `:dim()`,
`:italic()`, `:underline()`, `:blink()`, `:reverse()`, `:hidden()`,
`:crossed()` y `:reset()` (los colores son nombres de tema como `"red"`,
`"darkgray"` o `"#rrggbb"`).

```lua
local ui = pairee.ui
ui.Style():fg("cyan"):bold()                     -- estilo reutilizable
ui.Span("texto", "red")                          -- texto con estilo (estilo o color)
ui.Line({ "normal ", ui.Span("negrita"):bold() }):align(ui.Align.CENTER)
ui.Text("línea 1\nlínea 2")                      -- también un Line, Span o lista de ellos
ui.Text.parse("\27[31mrojo\27[0m")               -- conserva los colores ANSI
ui.Paragraph(texto):wrap(ui.Wrap.YES):scroll(2):block(ui.Border(ui.Edge.ALL))
ui.List({ "a", ui.Span("b"):fg("yellow") })
ui.Table({ ui.Row({ "a.txt", "12" }), { "b.txt", "7" } })
    :header({ "Nombre", "Tamaño" })
    :widths({ ui.Constraint.Length(20), ui.Constraint.Fill(1) })
ui.Gauge():percent(40):label("40 %"):gauge_style(ui.Style():fg("green"))
ui.Border(ui.Edge.TOP + ui.Edge.BOTTOM):type(ui.Border.ROUNDED):title("Info")
ui.Layout():direction(ui.Layout.HORIZONTAL):margin(1)
    :constraints({ ui.Constraint.Percentage(30), ui.Constraint.Fill(1) })
    :children({ widget_izquierdo, widget_derecho })
```

Constantes: `ui.Align.{LEFT,CENTER,RIGHT}`, `ui.Wrap.{NO,YES,TRIM}`,
`ui.Edge.{NONE,TOP,RIGHT,BOTTOM,LEFT,ALL}`,
`ui.Border.{PLAIN,ROUNDED,DOUBLE,THICK}`, `ui.Layout.{HORIZONTAL,VERTICAL}`;
`ui.Constraint.{Min,Max,Length,Percentage,Ratio,Fill}`. El layout es
declarativo: Pairee divide el área de vista previa al dibujar, por eso no hay
`split(rect)`. Las formas de la 1.0 (`ui.Gauge(ratio, label)`,
`ui.Table(headers, rows)`, `ui.Span(text, color)`, tablas con campo `type`)
siguen funcionando. Un widget admite como mucho 100 000 nodos y 8 MiB de texto.

### Valores tipados — `Path`, `Url`, `Cha`, `Error`

```lua
local p = pairee.Path("docs/guia.md")    -- o pairee.Path.os(...)
p.name, p.stem, p.ext, p.parent, p.is_absolute, p.has_root
p:join("x"), p:starts_with("docs"), p:ends_with("guia.md"), p:strip_prefix("docs")

local u = pairee.Url("sftp://yo@host//srv/www/index.html")
u.scheme, u.domain, u.path, u.name, u.parent, u.is_regular   -- "sftp", "yo@host", ...
u:join("css"), u:starts_with("sftp://yo@host/srv")           -- compara también esquema y host

local cha, err = pairee.fs.cha(ruta, seguir)   -- Cha, nil  |  nil, Error
cha.is_dir, cha.is_link, cha.is_orphan, cha.is_hidden, cha.is_exec, cha.len
cha.mtime, cha.atime, cha.btime, cha.mode, cha.uid, cha.gid, cha.nlink
cha:perm()                                    -- "drwxr-xr-x" (nil en Windows)
cha:hash()                                    -- resumen hex de tamaño y fechas
file.cha                                      -- lo mismo para un File

pairee.Error.custom("mensaje")                -- err.kind, err.code, err.message
```

`Path` y `Url` nunca tocan el disco. Toda función de `pairee.fs` acepta también
un `Path` o una `Url` local. Las APIs nuevas (`fs.cha`, `clipboard`, `state`,
`image`, `preview_code`, `preview_widget`) devuelven `valor, nil` si van bien y
`nil` (o `false`), `Error` si fallan; `err.kind` es un tipo de `std::io` como
`"NotFound"` o `"PermissionDenied"`. Las APIs anteriores siguen lanzando errores.

### `pairee.clipboard`, `pairee.state`, identidad

```lua
pairee.clipboard("texto")            -- true | false, Error  (todos los complementos)
local texto, err = pairee.clipboard() -- solo complementos de confianza fuera del Modo Seguro

pairee.state.set("contador", 3)      -- true | false, Error  (nil borra)
pairee.state.get("contador")         -- 3, se conserva entre llamadas y sesiones
pairee.state.all(), pairee.state.clear()

pairee.utils.user_name(), pairee.utils.host_name()
pairee.utils.uid(), pairee.utils.gid(), pairee.utils.group_name()  -- nil en Windows
```

`pairee.state` guarda valores compatibles con JSON (escalares, listas, tablas
con claves de texto) en `state.json` del directorio de datos del complemento,
256 KiB como máximo; un cambio que lo superaría se rechaza. El texto puesto en
el portapapeles pierde los caracteres de control salvo tabulador y salto de
línea y se limita a 1 MiB.

### `pairee.rt`, `pairee.th`

```lua
pairee.rt.version, pairee.rt.api, pairee.rt.os, pairee.rt.family
pairee.rt.plugin.name, .dir, .data_dir, .trusted, .secure_mode
pairee.rt.mgr.show_hidden, .sort_by, .sort_reverse
pairee.rt.preview.image, .max_bytes
pairee.rt.term.size()                 -- columnas, filas

pairee.th.name                        -- tema activo
pairee.th.header_fg                   -- ui.Style de un color del tema (las claves `*_bg` fijan el fondo)
```

### `pairee.image` y ayudas de vista previa

```lua
pairee.image.show(job.file)            -- widget de imagen (se dibuja como las imágenes de la Vista Rápida)
pairee.image.info(ruta)                -- { w, h, format, color }
pairee.image.precache(origen, destino, { max_width = 512, max_height = 512 })  -- miniatura PNG
pairee.preview_code({ file = job.file, skip = job.skip, lines = job.area.h })
pairee.preview_widget({ file = ruta, skip = 0 }, widget)  -- mostrar un widget más tarde
```

`image.show`, `image.info` y `preview_code` aceptan el archivo que se está
previsualizando (el archivo del job de `peek` / `preload` / `seek`) aunque esté
fuera de la jaula de `pairee.fs` del complemento; `precache` necesita un
origen que la jaula permita. `preview_code` muestra el texto con números de
línea, resaltado como en el visor integrado cuando el resaltado de sintaxis
está activo y una gramática reconoce el archivo.

### `pairee.ps` — Pub/Sub

```lua
pairee.ps.sub(event, fn)               -- suscribirse al evento
pairee.ps.pub(event, data)             -- publicar evento
pairee.ps.unsub(event)                 -- cancelar suscripción
```

### `pairee.log` — API de Registro de Bitácora

```lua
pairee.log.info(msg)                   -- Registrar mensaje a nivel info
pairee.log.warn(msg)                   -- Registrar mensaje a nivel warn
pairee.log.error(msg)                  -- Registrar mensaje a nivel error
pairee.log.debug(msg)                  -- Registrar mensaje a nivel debug
```

---

## 6. Comandos y atajos de teclado

Un plugin declara sus comandos en `manifest.toml`, cada uno con las teclas que
sugiere. Pairee los suma al mismo keymap que sus acciones propias: funcionan
con secuencias y con la tecla `<leader>`, aparecen en la paleta de comandos y
en la lista de atajos, y el usuario puede reasignarlos.

```toml
# En manifest.toml
[[commands]]
id    = "toggle"               # id en el keymap: plugin.<nombre del plugin>.toggle
title = "Alternar blame"       # se muestra en la paleta y en la lista de atajos

[commands.keys]                # por preset; "default" para el resto
default  = "Ctrl+Alt+b"
neovim   = "<leader>gb"
yazi     = "g b"
standard = "Alt+Shift+B"
```

Al pulsar la tecla se llama a `entry(args)`; `args.command` (y `args[1]`) es
el id del comando, así un mismo `entry` atiende varios comandos:

```lua
function M:entry(args)
    if args.command == "toggle" then
        -- ...
    end
end
```

**Las teclas son sugerencias.** Una tecla que el preset activo ya usa queda
sin asignar para el plugin (Pairee mantiene los atajos del usuario); si dos
plugins sugieren la misma tecla, la recibe el plugin cuyo nombre va primero
alfabéticamente. Los comandos sin tecla siguen en la paleta. El usuario
reasigna cualquier comando en `keybindings.toml`:

```toml
[overrides.all]
"plugin.git-blame.toggle" = "Ctrl+Alt+g"
```

Para evitar conflictos conviene usar `<leader>p…` en Neovim, `Alt+p` y una
letra en yazi, y `Ctrl+Alt+e` y una letra (`Ctrl+Alt+e h`) en los demás
presets, donde está libre. Evita `Ctrl+Alt+x` (vista de rama) y `Ctrl+Alt+p`
(paleta de comandos), y `Alt+p` fuera de yazi (vista rápida en standard).
`pairee developer lint` lista, por preset, las teclas sugeridas que están
ocupadas o no son válidas.

Escritura de teclas: `Ctrl+h`, `ctrl-h` y `<C-h>` son el mismo chord; una
letra mayúscula significa Shift (`Ctrl+H`); un espacio separa las teclas de
una secuencia (`g b`).

**Leer el keymap desde Lua.** `pairee.keymap.list()` devuelve cada binding
activo como `{ chord, id, label, origin }`; `pairee.keymap.chord_for(id)`
devuelve el primer chord de una acción (`"copy"`,
`"plugin.<nombre>.<comando>"`) o `nil`. **Disparar acciones.**
`pairee.emit(id)` ejecuta una acción del keymap como si se pulsara su
tecla (`pairee.emit("go_to_tab_2")`); `cd` y `focus` aceptan además
argumentos. Lo que un complemento puede disparar depende de su confianza:

* Los complementos **no confiables** solo pueden disparar acciones de
  navegación, vista, orden, selección y ayuda (`move_down`, `go_parent`,
  `go_to_tab_<n>`, `sort_by_name`, `select_all`, `quick_view`, `help`, ...) y
  sus **propios** comandos (`plugin.<este complemento>.<comando>`). Todo lo que
  copia, mueve, borra, escribe, ejecuta comandos, cambia la configuración o se
  conecta se rechaza con un diálogo de error.
* Los complementos **confiables** pueden disparar cualquier acción. Se
  ejecuta exactamente como la pulsación de la tecla, así que se aplican los
  diálogos de confirmación habituales (y la configuración de confirmaciones
  del usuario, como `confirm_delete`).
* `cd` (y `pairee.app.cd`) resuelve una ruta relativa respecto de la carpeta
  del panel activo y solo acepta una carpeta local existente; se rechaza en
  paneles SSH y de archivos comprimidos.
* Los argumentos se convierten a JSON con límites: las tablas deben ser listas
  densas (`{ "a", "b" }`) o mapas con claves de texto, con a lo sumo 32
  niveles de anidamiento y 100 000 valores en total. Las tablas dispersas,
  mixtas o cíclicas lanzan un error de Lua.

La tabla anterior `[keybindings] "ctrl+h" = "toggle"` sigue funcionando: cada
valor pasa a ser un comando con esas teclas como predeterminadas.

## 7. Escribir un Complemento Previewer

Los previewers implementan `peek(job)` y opcionalmente `preload(job)` y
`seek(job)`. Cuando la Vista Rápida muestra un archivo, Pairee llama a
`preload` de cada complemento que lo tenga y luego a `peek` de cada previewer
por orden de nombre; el primer widget devuelto sustituye a la vista previa
integrada (devuelve `nil` para declinar). Después se precargan en segundo
plano los tres archivos siguientes del panel.

El parámetro `job` provee:
- `job.file` — un `File` del archivo previsualizado (`.path`, `.name`, `.mime`, `.size`, `.cha`, ...)
- `job.area` — el área de vista previa en celdas: `x`, `y`, `w`, `h` (también `width`, `height`)
- `job.skip` — líneas desplazadas
- `job.units` — (solo `seek`) líneas a desplazar, negativo hacia arriba

`preload(job)` prepara algo por adelantado (una miniatura, un archivo de caché)
y devuelve `completo[, err]`; una versión de archivo cuyo preload devolvió
`true` no se vuelve a precargar. `seek(job)` devuelve el nuevo `skip`; Pairee
vuelve a llamar a `peek` con él. Sin `seek`, RePág / AvPág desplazan el propio
widget.

```lua
local M = {}

function M:peek(job)
    if not job.file.url:match("%.csv$") then
        return  -- no es un archivo CSV, declinar
    end

    local content = pairee.fs.read(job.file.url)
    local rows = {}
    for line in content:gmatch("[^\n]+") do
        local cols = {}
        for col in line:gmatch("[^,]+") do
            cols[#cols + 1] = col
        end
        rows[#rows + 1] = cols
    end

    local headers = table.remove(rows, 1)
    return pairee.ui.Table(rows):header(headers):style(pairee.th.panel_fg)
end

return M
```

Un previewer no confiable no puede leer con `pairee.fs` archivos de fuera de su
carpeta, pero sí puede mostrar el archivo previsualizado a través del anfitrión:

```lua
function M:peek(job)
    if job.file.mime:match("^image/") then
        return pairee.image.show(job.file)
    end
    return pairee.preview_code({ file = job.file, skip = job.skip, lines = job.area.h })
end
```

---

## 8. Escribir un Complemento Hook

Los complementos hook se suscriben a eventos de ciclo de vida durante su llamada `setup(opts)`.

**Eventos disponibles:**

| Evento | Payload | Se dispara Cuando |
|--------|---------|-----------------|
| `on_cd` | `{ path: string, side: string }` | Un panel entra en una carpeta. `side`: `left` / `right` |
| `on_hover` | `{ path: string, is_dir: boolean, size: integer }` | La vista rápida muestra otra entrada (la del cursor, o la primera seleccionada); solo con la vista rápida abierta |
| `on_key` | `{ key: string }` | Se presiona una tecla (antes del resolver) |
| `on_mouse` | `{ kind, button, x, y, target, side?, index? }` | Un clic en los paneles o las barras (no dentro de diálogos). `kind`: `click` / `double_click`; `button`: `left` / `right` / `middle`; `target`: `panel_row`, `panel`, `panel_title`, `panel_header`, `tab`, `menu`, `fkey`, `command_line`, `other`; `side`: `left` / `right`; `index`: entrada (desde 1) de un `panel_row` |

```lua
local M = {}

function M:setup(opts)
    pairee.ps.sub("on_cd", function(payload)
        pairee.log.info("Navegado a: " .. payload.path)
        local result = pairee.fs.spawn("git", { "-C", payload.path, "status", "--short" })
        if result.status == 0 and result.stdout ~= "" then
            pairee.app.notify("Git Status", result.stdout, "info")
        end
    end)
end

return M
```

---

## 9. Escribir un Complemento Comando

Los complementos comando implementan `entry(args)`. Se ejecutan con sus teclas o desde la paleta de comandos (ver la sección 6).

```lua
local M = {}

function M:entry()
    local result = pairee.fs.spawn("fzf", { "--layout=reverse", "--height=40%" })
    if result.status == 0 and result.stdout ~= "" then
        local target = result.stdout:gsub("\n$", "")
        pairee.app.cd(target)
    end
end

return M
```

---

## 10. Sincronización de Estado con `pairee.sync`

El código de los complementos se ejecuta asincrónicamente en hilos de segundo plano. Leer `AppState` directamente desde un contexto asíncrono no es seguro. Usa `pairee.sync()` para recibir un snapshot de solo lectura:

```lua
-- Envuelve la función de lectura de estado al momento de la configuración
local leer_estado = pairee.sync(function()
    return {
        cwd   = pairee.app.cwd(),
        focus = pairee.app.focus(),
    }
end)

-- Llámala más tarde en contexto asíncrono
local estado = leer_estado()
print(estado.cwd)
```

**Reglas:**
- `pairee.sync()` debe llamarse al cargar el complemento (en el nivel superior o dentro de `setup`), no dentro de callbacks asíncronos.
- El callable devuelto es seguro para llamar desde cualquier contexto asíncrono.

### Anotaciones

El bloque de comentarios inicial de `main.lua` puede llevar anotaciones:

```lua
--- @since 1.2.0
--- @sync entry
local M = {}
```

- `--- @since <versión>` — la versión de la API Lua que necesita el complemento;
  un Pairee con una API anterior se niega a cargarlo (con ese motivo en el log)
  en lugar de fallar más tarde por una función que no existe.
- `--- @sync entry|peek|preload|seek` — antes de llamar a ese método, Pairee
  refresca `pairee.cx` (como hace `pairee.sync`), así que lee los paneles tal
  como están en el momento de la llamada.

Todos los métodos del complemento (`setup`, `entry`, `peek`, `preload`, `seek`
y los callbacks de `pairee.ps`) se ejecutan de forma asíncrona, así que pueden
esperar diálogos, `pairee.sync`, `pairee.utils.sleep` o procesos hijos.

---

## 11. Eventos Pub/Sub con `pairee.ps`

Los complementos pueden comunicarse entre sí a través de canales pub/sub con nombre:

```lua
-- Complemento publicador
pairee.ps.pub("mi-complemento:resultado", { path = "/alguna/ruta" })

-- Complemento suscriptor
pairee.ps.sub("mi-complemento:resultado", function(data)
    pairee.app.cd(data.path)
end)
```

Usa nombres de eventos con espacio de nombres (`nombre-complemento:evento`) para evitar conflictos.

---

## 12. Depuración y Control de Errores

Pairee proporciona ricas características de aislamiento y registro de bitácora:

### 12.1 API de Registro de Bitácora (`pairee.log`)
Utiliza las funciones de registro para escribir información o marcadores de depuración directamente en el archivo de registro de Pairee ubicado en `~/.cache/pairee/app.log`:
```lua
pairee.log.debug("Verificando la estructura del CSV...")
pairee.log.info("La vista previa del CSV se renderizó correctamente.")
pairee.log.error("Error al analizar la fila: " .. tostring(err))
```

### 12.2 Interceptación de Errores en Tiempo de Ejecución
* Cualquier excepción en tiempo de ejecución (como errores de sintaxis, acceso a campos nil o división por cero) es capturada por el gestor de Lua de Pairee.
* Estas excepciones se interceptan y muestran como una notificación visual de error en la UI que indica el nombre del complemento, el número de línea y el mensaje de error específico.
* Se escribe un rastreo de pila completo (backtrace) en `app.log` para ayudarte a depurar.

### 12.3 Modo de Depuración de la CLI
Para ejecutar Pairee en modo de depuración y rastrear la ejecución de tu complemento en vivo, lanza el ejecutable desde una terminal independiente con:
```bash
pairee --plugin-debug <nombre-complemento>
```
Todas las llamadas a `pairee.log.*` y los errores en tiempo de ejecución de Lua de ese complemento se enviarán directamente a la salida estándar (stdout) de la terminal desde la cual se inició el modo depuración.

---

## 13. Aislamiento (Sandboxing), Modo Confiable y Protección del Modo Seguro

Para proteger a los usuarios de complementos maliciosos o con fallos, Pairee implementa un límite de seguridad estricto y de múltiples capas.

### 13.1 La Máquina Virtual de Aislamiento (Modo No Confiable por Defecto)
Por defecto, todos los complementos se ejecutan en **Modo No Confiable** (`trusted = false`), lo que los aísla dentro de una máquina virtual de sandbox segura:
* **Bibliotecas Bloqueadas:** El complemento no puede acceder a las bibliotecas estándar de Lua `io`, `os`, `package`, `coroutine` o `debug`.
* **Funciones Dinámicas Prohibidas:** Las funciones globales que permiten la evaluación dinámica de código (`load`, `loadstring`, `dofile`, `loadfile`) están deshabilitadas.
* **Aislamiento de Require:** `require("lib.util")` carga `lib/util.lua` (o `lib/util/init.lua`) de la carpeta del propio complemento, lo ejecuta una vez y guarda el resultado en una tabla privada del complemento. Funciona igual en complementos confiables y no confiables (no necesita la biblioteca `package`). Los nombres de módulo son identificadores con puntos (letras, dígitos, `_`, `-`); ni `..`, ni separadores, ni enlaces simbólicos o junctions pueden llegar a archivos fuera de la carpeta del complemento.
* **Sin Comandos Externos:** Cualquier llamada a `pairee.fs.spawn()` generará inmediatamente un error de ejecución.
* **Jaula del Sistema de Archivos:** `pairee.fs` solo puede leer dentro del directorio del propio complemento y de su directorio de datos privado (`pairee.fs.data_dir()`), y solo puede escribir dentro del directorio de datos. Esto se aplica tanto si el Modo Seguro está activo como si no; los segmentos `..` y los enlaces simbólicos no permiten escapar de ella.
* **Excepción de vista previa:** `pairee.image.show`, `pairee.image.info` y `pairee.preview_code` aceptan también el archivo que se está previsualizando; se lo pasan al anfitrión y devuelven un widget opaco (o el tamaño de la imagen), nunca su contenido. `pairee.image.precache` decodifica píxeles que el complemento puede volver a leer, así que solo admite orígenes dentro de la jaula.
* **Portapapeles:** un complemento no confiable puede escribir en el portapapeles pero no leerlo (leer requiere un complemento de confianza fuera del Modo Seguro).

```toml
# En config.toml
[plugins.csv-preview]
name    = "csv-preview"
trusted = false       # No requiere acceso al sistema (modo sandbox no confiable)

[plugins.git-status]
name    = "git-status"
trusted = true        # Solicita permisos para ejecutar herramientas externas (modo confiable)
```

### 13.2 Modo Confiable (`trusted = true`)
Cuando un complemento es marcado explícitamente como confiable por el usuario, se ejecuta en **Modo Confiable**:
* El complemento puede utilizar la gestión de archivos y módulos estándar de Lua (`io`, `os`, `package`, `load`), solo fuera del Modo Seguro (13.3).
* El complemento puede ejecutar comandos del sistema a través de la API `pairee.fs.spawn()`.

### 13.3 Modo Seguro Global (`secure_mode = true`)
Para evitar la filtración de datos, los usuarios pueden habilitar un **Modo Seguro** global en su configuración principal. Este modo actúa como un cortafuegos a nivel de motor:
* **Activación:** Se configura en `config.toml` como `secure_mode = true` (o en **Configuración → Lenguaje y Plugins → Modo Seguro para todos los plugins**). Esta configuración es de solo lectura en tiempo de ejecución y completamente inmutable desde Lua.
* **Biblioteca estándar restringida:** un complemento confiable recibe las mismas bibliotecas de Lua que uno no confiable, más `coroutine` y un `os` reducido al reloj (`os.time`, `os.date`, `os.clock`, `os.difftime`): sin `io`, `package`, `load` / `loadfile` / `dofile`, `os.execute`, `os.remove`, `os.rename`, `os.getenv` ni `os.exit`, que saltearían la jaula y la lista de procesos permitidos de abajo.
* **Prohibición de Red y Sockets:** La creación de sockets TCP/UDP y las solicitudes HTTP están completamente deshabilitadas a nivel de motor para todos los complementos, incluso aquellos con `trusted = true`.
* **Lista Permitida de Procesos:** En Modo Seguro, un complemento confiable solo puede ejecutar (`pairee.Command`, `pairee.fs.spawn`) los programas que declara en su manifiesto:
  ```toml
  [permissions]
  commands = ["git", "rg"]
  ```
  * El programa debe indicarse por **nombre simple** (sin ruta absoluta ni relativa); se resuelve mediante las entradas absolutas de `PATH` y se ejecuta esa ruta absoluta. En Windows solo se resuelven `.exe` / `.com` (nunca `.bat` / `.cmd`).
  * Los shells, intérpretes, utilidades de red y envoltorios de comandos (`sh`, `bash`, `cmd`, `powershell`, `pwsh`, `python*`, `node`, `perl`, `ruby`, `lua`, `curl`, `wget`, `ssh`, `env`, `xargs`, `sudo`, ...) se rechazan **siempre**, aunque estén declarados, y también cuando un nombre declarado es un enlace simbólico a uno de ellos.
  * Los comandos no declarados fallan con `Security violation: ... is not declared in the plugin manifest`. El Gestor de Complementos muestra los comandos declarados en los detalles del complemento para que el usuario los revise antes de confiar en él.
* **Entorno y directorio de trabajo:** `Command:env` se limita a variables de idioma/terminal/color y `Command:cwd` a carpetas dentro de la jaula (ver `pairee.Command`), para que una herramienta declarada no se convierta en un ejecutor de código mediante `GIT_SSH_COMMAND`, `LD_PRELOAD`, `PAGER` y variables similares.
* **Argumentos:** se rechazan las opciones que hacen que una herramienta declarada ejecute otro programa (claves de configuración de Git que nombran programas con `-c` o `git config`, `--upload-pack`, `--exec`, `--git-dir`, `rebase -x`, `difftool -x`, `grep -O`, `submodule foreach`, `bisect run`, `filter-branch`, `send-email`; `tar --to-command` / `-I`; `rg --pre`; `fd --exec`; `fzf --preview`; ...). `git` siempre se ejecuta con `-c safe.bareRepository=explicit` antes de los argumentos del complemento.
* **Sandboxing del Sistema de Archivos:** Para complementos confiables, las **lecturas** de archivos (`pairee.fs`) están restringidas al espacio de trabajo activo, a las carpetas de configuración y caché del usuario, y a los directorios propio y de datos del complemento. Las **escrituras** se limitan al espacio de trabajo, la caché y el directorio de datos del complemento: la carpeta de configuración de Pairee (`config.toml`, `keybindings.toml`, asociaciones, el menú de usuario, `plugins/`, `plugins.lock`) es de solo lectura, así que un complemento no puede desactivar el Modo Seguro ni marcarse como confiable. Nada dentro de una carpeta `.git` se puede escribir (su `config` y sus hooks ejecutan programas). Los complementos no confiables mantienen su jaula más estricta (13.1).
* **Falla cerrado:** si `config.toml` no se puede leer, los complementos se ejecutan en Modo Seguro hasta que se corrija (el error de inicio lo indica).

### 13.4 Límites de recursos
Cada complemento tiene un límite de memoria (128 MiB no confiable, 512 MiB confiable) y un watchdog de CPU: cada llamada que Pairee hace al complemento (`main.lua`, `setup`, `entry`, `peek`, callbacks de hooks) puede durar como máximo 10 segundos, incluido el tiempo dentro de funciones C como `string.rep`. El reloj solo corre mientras el complemento se ejecuta: mientras una llamada espera un diálogo, `pairee.sync`, una pausa o un proceso hijo, se detiene. Una llamada que se pasa del presupuesto se aborta con `plugin aborted: exceeded its execution time limit` y `pcall` no puede capturarlo; la siguiente llamada empieza con un presupuesto nuevo.

### 13.5 Gestionar complementos
En el Gestor de Complementos (pestaña Instalados) `c` o Enter abre los ajustes del complemento (sección 18), `t` alterna la confianza de un complemento, `e` lo activa o desactiva y `s` alterna el Modo Seguro global. Los cambios se guardan y se aplican de inmediato: todos los complementos se recargan con la nueva configuración. Los complementos desactivados no se cargan, así que no se ejecuta nada de su código ni de sus hooks.

---

## 14. Probar tu Complemento Localmente

1. **Coloca la carpeta de tu complemento** bajo `~/.config/pairee/plugins/<nombre>.pairee/`; el `name` del manifiesto debe ser `<nombre>`.
2. **Regístralo** en `config.toml` (hace falta para `trusted = true`; un complemento sin registrar carga como no confiable).
3. Ejecuta Pairee y revisa los registros en `~/.cache/pairee/app.log` para ver los eventos de ejecución.

---

## 15. Herramientas del Modo Desarrollador y TUI de Envío de PR

Para simplificar el empaquetado y la validación de los complementos, los desarrolladores pueden activar el modo desarrollador: **Configuración → Lenguaje y Plugins → Modo desarrollador de plugins**, que se guarda en `config.toml` como:

```toml
# En config.toml
plugins_developer_mode = true   # Muestra la pestaña Developer Tools en el menú de plugins
```

Los comandos de CLI `pairee developer` de abajo funcionan con o sin esta opción; solo habilita las herramientas de la TUI (15.2).

### 15.1 Comandos de Desarrollador en CLI
Se ejecutan desde dentro de la carpeta del complemento (actúan sobre el directorio actual):
* `pairee developer init <nombre>`: Crea un complemento nuevo a partir de la rama `plugin-template` (15.4); pide una descripción y un autor.
* `pairee developer lint`: Comprueba que `manifest.toml` se pueda leer y defina `default_language`, que exista `main.lua`, avisa del uso de `os.execute`, `io.open`, `os.system`, `dofile` y `loadfile` en un complemento sin `requires_trust = true`, e informa las teclas sugeridas inválidas o en conflicto en cada preset.
* `pairee developer package`: Ejecuta las comprobaciones de publicación (15.3) y empaqueta el complemento en un clon local de la rama `plugin-registry`:
  * **Genera Hashes:** Genera/actualiza automáticamente el archivo de hashes de integridad `sha256.sum` para todos los archivos de la carpeta. (Nota: esto no crea un archivo comprimido; los complementos en el registro de Pairee se distribuyen como carpetas abiertas).
  * **Detección y Generación de Licencia:**
    - Busca un archivo de licencia `LICENSE` (sin distinguir mayúsculas/minúsculas) en la raíz del plugin.
    - Si el archivo existe pero el campo `license` en `manifest.toml` está vacío, el asistente solicita ingresar el nombre de la licencia (o asigna `"Custom"` en entornos no interactivos).
    - Si no existe ningún archivo de licencia, asigna automáticamente la licencia `"MIT"` en el manifiesto y genera un archivo `LICENSE` con la licencia MIT estándar y los datos de derechos de autor en el espacio de trabajo del plugin.

### 15.2 Herramientas de Desarrollo en TUI y Asistente de PR
Las herramientas interactivas de desarrollo en TUI (accesibles a través de **Barra de menú (F9) → Archivos → Comandos de plugins** → pestaña `Developer Tools` cuando el modo desarrollador está activo) ofrecen el siguiente conjunto de utilidades:

* **Seleccionar Plugin Activo (Opción 0):** Abre un modal interactivo con la lista de plugins detectados en el sistema. Enumera todos los plugins encontrados bajo `plugins_dev_dir` y escanea las rutas actuales de los paneles activos (Panel 1 y Panel 2) en busca de un archivo `manifest.toml`, permitiéndote seleccionar tu objetivo de desarrollo directamente con el teclado.
* **Inicializar plantilla (Opción 1):** Guía paso a paso al desarrollador para crear un nuevo esqueleto de plugin en el directorio de desarrollo. Al completarse con éxito, el nuevo plugin se selecciona automáticamente en la Opción 0 y esta opción queda deshabilitada.
* **Verificar (Lint) (Opción 2):** Escanea la carpeta del plugin activo y ejecuta comprobaciones de sintaxis y seguridad tanto en los archivos Lua como en el manifiesto.
* **Empaquetar (Opción 3):** Prepara un clon local temporal de la rama `plugin-registry`, copia todos los recursos del plugin, construye/actualiza las entradas de catálogo en el archivo maestro de índice y muestra en pantalla la ruta absoluta exacta de la caché local donde se empaquetó.
* **Instalar localmente (Opción 4):** Copia el plugin activo directamente al directorio local de plugins de Pairee y lo registra en `plugins.lock` para poder probarlo de forma inmediata en la interfaz.
* **Enviar plugin (GitHub PR) (Opción 5):** Asistente que recopila descripciones, realiza commits git locales y opcionalmente utiliza un token PAT de GitHub para subir los cambios a un fork y abrir una Pull Request de forma automatizada.

### 15.3 Guía de Resolución de Problemas de las Comprobaciones de Publicación
`pairee developer package`, `pairee developer submit` y las opciones **Package** / **Submit** de la TUI se detienen en la primera comprobación que falla. El mensaje se muestra tal cual (no hay códigos de error):

| Mensaje | Causa | Solución |
|---------|-------|----------|
| `No se encontró manifest.toml en el directorio actual...` | El comando se ejecutó fuera de la carpeta del complemento. | Entra primero (`cd`) en la carpeta del complemento. |
| `Error al leer manifest.toml` / `Error al analizar manifest.toml` | El manifiesto no se puede leer, no es TOML válido o le faltan campos obligatorios. | Corrige la sintaxis TOML y los campos de la sección 20. |
| `Nombre o autor de plugin no válido '...'` | `name` o `author` contiene algo distinto de letras, dígitos, `_` y `-`. | Renómbralo; la carpeta del registro es `plugins/<shard>/<autor>/<nombre>/`. |
| `icon.png no encontrado en la raíz del plugin...` | Falta `icon` en el manifiesto o el archivo no existe. | Agrega un PNG y apunta `icon` a él. |
| `el tamaño del icono debe ser de 256x256 o 512x512 píxeles` | El icono tiene otro tamaño. | Redimensiónalo a 256x256 o 512x512 exactos. |
| `el formato de imagen del icono no es válido o está dañado` | El archivo no es una imagen legible. | Vuelve a exportarlo como PNG. |
| `el directorio screenshots/ no contiene ninguna captura...` | Falta el array `screenshots` o está vacío. | Lista al menos una imagen PNG/JPG/JPEG. |
| `captura de pantalla '...' no encontrada` | Una captura listada no existe. | Corrige la ruta (relativa a la raíz del complemento). |
| `el tamaño de la captura '...' debe ser de al menos 640x480 píxeles` | Una captura es demasiado chica. | Usa una captura de al menos 640x480. |
| `el formato de imagen de la captura '...' no es válido o está dañado` | El archivo no es una imagen legible. | Vuelve a exportarla. |

La sintaxis Lua, la cobertura de traducciones y la alineación de claves no se comprueban al empaquetar: ejecuta `pairee developer lint` y prueba el complemento localmente (sección 14). `scripts/check_translations.py` solo cubre los archivos `lang/` del propio Pairee.

### 15.4 Rama de Plantilla de Complementos (`plugin-template`)

Pairee mantiene una rama git huérfana dedicada llamada **`plugin-template`** en su propio repositorio. Esta rama contiene los archivos de código base canónicos y siempre actualizados para nuevos complementos, y es completamente invisible para los usuarios finales: nunca aparece en ningún listado de plugins dentro de la TUI.

#### Cómo Funciona

Cuando un desarrollador usa el asistente **Inicializar Nuevo Complemento** (ya sea desde la pestaña Herramientas de Desarrollador de la TUI o `pairee developer init`), la herramienta:

1. **Localiza el repositorio de Pairee** en disco recorriendo hacia arriba desde la ubicación del binario en ejecución hasta encontrar un directorio `.git`. Alternativamente, la variable de entorno `PAIREE_REPO_DIR` puede configurarse para apuntar al repositorio de forma explícita.
2. **Extrae los archivos** directamente de la rama `plugin-template` usando `libgit2` (sin necesidad de tener `git` instalado como comando externo). Cada archivo de la rama se escribe al directorio de destino del nuevo complemento.
3. **Sustituye los marcadores de posición** en `manifest.toml` y `help/en.md`:
   - `PLUGIN_NAME` → el nombre ingresado por el desarrollador
   - `PLUGIN_DESCRIPTION` → la descripción ingresada por el desarrollador
   - `PLUGIN_AUTHOR` → el autor ingresado por el desarrollador
4. Si la rama o el repositorio **no están disponibles** (ej. instalación como binario independiente sin el repo fuente), el sistema recurre de forma transparente a generar los archivos desde las cadenas de localización integradas — garantizando que la inicialización de plugins funcione siempre.

#### Estructura de Archivos de la Rama Template

```
plugin-template/
├── manifest.toml          # Bloque [plugin] con tokens PLUGIN_NAME, PLUGIN_DESCRIPTION, PLUGIN_AUTHOR,
│                          # license, type, min_pairee = "0.11.0", una entrada [[commands]] y settings_schema
├── main.lua               # Código Lua completo con setup(), entry() y peek()
├── icon.png               # PNG de marcador de posición 256×256 en gris
├── lang/
│   └── en.toml            # Claves de traducción en inglés por defecto
├── help/
│   └── en.md              # Archivo de ayuda con token PLUGIN_NAME
└── screenshots/
    └── screenshot1.png    # PNG de marcador de posición 640×480 en gris
```

#### Variable de Entorno PAIREE_REPO_DIR

Configura esta variable para indicarle a Pairee dónde encontrar su repositorio fuente cuando la detección automática falla:

```sh
export PAIREE_REPO_DIR=/ruta/al/repositorio/pairee
pairee
```

#### Editar la Plantilla

Para actualizar los archivos de código base que usan todos los nuevos complementos al crearse, simplemente hacé checkout de la rama y editá los archivos directamente:

```sh
git checkout plugin-template
# editar archivos...
git add -A && git commit -m "feat: actualizar plantilla de plugin"
git checkout master
```

> **Nota:** Los cambios en la rama `plugin-template` **no afectan** a `master` y viceversa — la rama no comparte historial con el código base principal.

---

## 16. Flujo de Envío al Registro

1. **Activar el Modo Desarrollador** (`plugins_developer_mode = true` en `config.toml`).
2. Abrir la pantalla del **Asistente de Metadatos TUI** o ejecutar `pairee developer package` desde dentro de la carpeta del complemento para validar tus archivos, copiarlos a una copia de trabajo local de la rama `plugin-registry` y actualizar el archivo de catálogo central `registry/index.toml`.
3. Asegurarse de que el plugin cumpla con los siguientes requisitos obligatorios de publicación:
   - **Icono:** Debe existir un icono PNG llamado `icon.png` en el directorio raíz del plugin (tamaños recomendados: `512x512` o `256x256` píxeles).
   - **Capturas de pantalla:** Debe existir un directorio `screenshots/` en la raíz del plugin que contenga al menos una captura de pantalla (PNG, JPG o JPEG).
4. Ejecutar la opción de empaquetado para clonar y preparar la rama `plugin-registry` localmente, verificar todas las firmas, copiar los archivos de código y generar la suma de verificación y la entrada del índice central.
5. Ejecutar la opción "Enviar". El asistente te solicitará:
   - **Descripción del Commit:** Una descripción detallada de los cambios o características.
   - **Token de GitHub (Opcional):** Si se suministra, Pairee realizará automáticamente un fork del repositorio, subirá la rama y creará una Pull Request. Si se omite, Pairee realizará un commit git local e imprimirá los comandos manuales para que los copies y ejecutes para subir la rama y abrir la Pull Request por ti mismo.
6. **Las firmas las pone el registro, no tú.** No incluyas archivos `.minisig` en tu Pull Request. Cuando se fusiona, el workflow de firma del registro firma tu `manifest.toml` y un `index.toml` nuevo (con un número de serie y una fecha que Pairee comprueba para rechazar copias viejas o congeladas del índice) con la clave de Pairee. Los usuarios pueden instalar tu plugin una vez publicado ese índice firmado.

---

## 17. Localización de tu Complemento (Traducción)

Los complementos de Pairee admiten traducciones aisladas y autocontenidas. No es necesario modificar los archivos de localización principales de la aplicación.

### 17.1 Creación de Archivos de Idioma
Crea un subdirectorio `lang/` en la carpeta raíz de tu complemento y coloca allí archivos de traducción TOML nombrados con su código de idioma ISO-639-1:
```text
~/.config/pairee/plugins/mi-complemento.pairee/
├── manifest.toml
├── main.lua
└── lang/
    ├── en.toml        # Traducciones en inglés por defecto
    └── es.toml        # Traducciones en español
```

Dentro de los archivos de traducción (p. ej., `lang/es.toml`), escribe tus claves y mensajes correspondientes:
```toml
[messages]
hello = "¡Hola {name}!"
error_vcs = "Error al ejecutar comando Git"
```

En tu `manifest.toml`, especifica tu idioma predeterminado:
```toml
default_language = "en"
```

### 17.2 Uso de Traducciones en Lua
Utiliza la función global `pairee.t()` para traducir claves de forma dinámica:
```lua
-- Búsqueda de traducción simple:
local msg_error = pairee.t("messages.error_vcs")

-- Traducción con interpolación de variables:
local saludo = pairee.t("messages.hello", { name = "Iván" }) -- "¡Hola Iván!"
```

### 17.3 Mecánica de Fallbacks (Recuperación)
Cuando `pairee.t()` resuelve una clave:
1. Comprueba el idioma de interfaz activo del usuario en la aplicación Pairee (p. ej., español `es`).
2. Si `lang/es.toml` tiene la clave, la utiliza.
3. Si no la encuentra o si `lang/es.toml` no está disponible, recurre al idioma 
---

## 18. Configuración Personalizada de Complementos

Los complementos pueden declarar sus propios ajustes. Los usuarios los editan en el Gestor de Complementos sin tocar el código Lua: `c` o Enter sobre un complemento instalado abre un formulario con una casilla por cada ajuste `bool` y un campo de texto para los demás.

### 18.1 Definición de settings_schema
En el manifiesto `manifest.toml` de tu complemento, define una tabla `[settings_schema]` que contenga las opciones configurables. Cada opción debe especificar un tipo `type` (`bool`, `string`, `integer` o `number`), un valor por defecto `default` y una breve descripción `description`. El valor del usuario se convierte con el tipo declarado (sin `type`, se usa el tipo de `default`); un valor que no corresponde al tipo se ignora y se usa el valor por defecto:
```toml
[settings_schema]
show_hidden = { type = "bool", default = false, description = "Mostrar archivos ocultos de VCS" }
git_path    = { type = "string", default = "git", description = "Ruta personalizada al ejecutable Git" }
max_depth   = { type = "integer", default = 3, description = "Profundidad máxima de recursión" }
```

El formulario muestra la `description` y el valor por defecto del ajuste enfocado. Guardar comprueba cada valor contra su tipo (uno inválido se nombra y recibe el foco); Predeterminados devuelve todos los campos al valor por defecto del esquema. Solo se guardan los valores distintos de los predeterminados, en `config.toml` bajo `[settings.plugin_settings.<complemento>]`, y los complementos se recargan para que los nuevos valores se apliquen de inmediato.

### 18.2 Acceso a la Configuración en Lua
Los valores de configuración resueltos se ponen a disposición de tu script Lua dentro de la tabla global de solo lectura `pairee.settings`:
```lua
-- Leer configuraciones en main.lua:
local ocultos_visibles = pairee.settings.show_hidden
local limite_profundidad = pairee.settings.max_depth or 3

if ocultos_visibles then
  -- Realizar acción
end
```

---

## 19. Documentación de Ayuda del Complemento (Ayuda F1)

Para documentar atajos de teclado, comandos y funciones para los usuarios, puedes integrar tu documentación directamente en el visor de ayuda integrado `F1` de Pairee.

### 19.1 Creación de Archivos Markdown de Ayuda
Agrega un subdirectorio `help/` a la carpeta raíz de tu complemento, y coloca dentro los archivos markdown de ayuda nombrados según su código de idioma ISO-639-1:
```text
~/.config/pairee/plugins/mi-complemento.pairee/
├── manifest.toml
├── main.lua
└── help/
    ├── en.md          # Documentación en inglés (Fallback por defecto)
    └── es.md          # Documentación en español
```

Escribe el contenido utilizando markdown estándar:
```markdown
# Mi Complemento Git Status

Este complemento muestra la rama de Git actual y el estado de modificación en el encabezado del panel.

## Atajos de Teclado
* `Ctrl+G`: Actualizar estado de Git
* `Ctrl+Shift+G`: Confirmar (commit) cambios del panel actual

## Configuración
Habilita `show_hidden` en las opciones de Pairee para monitorear archivos ignorados por Git.
```

### 19.2 Visualización en la Interfaz de Usuario
Cuando los usuarios abren el menú de ayuda presionando `F1`, el sistema mostrará una pestaña o barra lateral llamada **"Ayuda de Complementos"** y listará todos los complementos activos que contengan archivos de ayuda válidos dentro de sus directorios `help/`. Al seleccionar tu complemento, se mostrará su documentación markdown formateada (resuelta según el idioma activo del usuario o el idioma por defecto) directamente en el panel principal.

---

## 20. Referencia del Manifiesto

```toml
name          = "mi-complemento"
version       = "1.0.0"
description   = "Descripción de mi complemento"
author        = "tu-usuario-github"
license       = "MIT"
type          = "hook"               # Autodetectado por la herramienta de desarrollador
min_pairee    = "0.7.0"
requires_trust = false

# Soporte de integración de idiomas
default_language = "en"
languages     = ["en", "es"]         # Autodetectado desde el directorio lang/

# Esquema de configuración personalizada
[settings_schema]
show_hidden = { type = "bool", default = false, description = "Mostrar archivos ocultos de VCS" }
git_path    = { type = "string", default = "git", description = "Ruta personalizada al ejecutable Git" }

[[commands]]
id = "run"
title = "Run"
[commands.keys]
default = "Ctrl+Alt+r"

# Programas que el complemento puede ejecutar con el Modo Seguro activo (nombres simples, ver 13.3)
[permissions]
commands = ["git"]
```

---

## 21. Mejores Prácticas y Convenciones

| Convención | Guía |
|------------|------|
| **Nombres de complementos** | Usa el estilo `<name>.pairee` para complementos del registro (p.ej. `git.pairee`, `fzf.pairee`) |
| **Eventos con espacio de nombres** | Prefija los eventos pub/sub con el nombre del complemento: `mi-complemento:evento` |
| **Declinar con gracia** | En `peek()`, verifica el tipo de archivo y retorna anticipadamente si no aplica |
| **Asíncrono primero** | Evita llamadas bloqueantes. Usa `pairee.Command` y patrones asíncronos; deja el trabajo lento de la vista previa para `preload` |
| **Notificaciones de error** | Muestra `pairee.app.notify` con nivel `"error"` en fallos — nunca falles silenciosamente |
| **Transparencia de confianza** | Si tu complemento necesita `trusted = true`, documéntalo claramente en el manifiesto |
| **Disciplina SemVer** | Incrementa MAJOR solo en cambios de comportamiento que rompen compatibilidad |
| **Superficie mínima** | Expón solo lo que el complemento necesita — menos superficie = menos vector de ataque |

---

*Véase también: [docs/technical/plugin-system-design-es.md](technical/plugin-system-design-es.md) para la arquitectura del motor Rust.*
*Véase también: [docs/technical/plugin-registry-spec.md](technical/plugin-registry-spec.md) para el esquema del registro y el flujo de envío.*
