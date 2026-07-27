# 📖 Referencia: API Lua de Plugins (`pairee.*`)

> **Cuadrante: REFERENCE** — *orientado a información.*

Esta página es la referencia completa de la API Lua expuesta a los
plugins de Pairee. Para el workflow del lado usuario (instalar, confiar,
actualizar), mirá [`28_howto_plugins`](28_howto_plugins.md). Para el
sandbox y modelo de confianza, mirá
[`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md).

> **Audiencia**: autores de plugins. El `main.lua` de un plugin es la
> única entry point; cada API descripta acá es alcanzable como
> `pairee.<name>` (o, para la familia legacy, `pairee.app.<name>`).

---

## 1. Namespace de nivel superior

| Entry | Propósito | Estabilidad |
| --- | --- | --- |
| `pairee.app` | Superficie de aplicación legacy. | Stable. `confirm` / `input` loguean un warning de deprecation. |
| `pairee.emit(action, args)` | Dispara cualquier acción registrada por nombre. | Stable. |
| `pairee.confirm({...})` | Abre un diálogo real de confirmación Y/N. | Stable. |
| `pairee.input({...})` | Abre un diálogo real de input. | Stable. |
| `pairee.which({cands, silent})` | Prompt para una de varias teclas. | Stable. |
| `pairee.notify({...})` | Muestra una notificación estructurada. | Stable. |
| `pairee.file_cache({file, skip})` | Path de caché estable para `(file, skip)`. | Stable. |
| `pairee.utils.*` | OS, time, hash helpers. | Stable. |
| `pairee.fs.*` | Operaciones de filesystem. | Stable. |
| `pairee.ui.*` | Constructores de widgets (`Paragraph`, `Gauge`, `List`, `Table`, `Span`, `Line`). | Stable; widgets userdata más ricos en un release posterior. |
| `pairee.ps.sub / pub / unsub` | Pub/sub local. | Stable. |
| `pairee.log.*` | Loguea un mensaje a un nivel. | Stable. |
| `pairee.sync(fn)` | Snapshot-bridge al main thread. | Stable. |
| `pairee.settings.*` | Read access a los settings resueltos del plugin. | Stable. |
| `pairee.t(key, vars)` | Lookup de string localizado con interpolación de variables. | Stable. |
| `pairee._secure_mode` | `true` cuando Secure Mode está activo. | Stable. |

---

## 2. `pairee.emit(action, args)`

La entry point única y unificada para disparar cualquier acción
registrada.

```lua
pairee.emit("cd", "/tmp")                  -- string arg
pairee.emit("cd", { path = "/tmp" })       -- table arg
pairee.emit("set_focus", "left")           -- alias: "focus" también funciona
pairee.emit("select", { url = f.url, state = true })
```

`args` se convierte a JSON (Lua table → JSON object, Lua table
integer-indexed → JSON array, scalar → scalar) y se forwardea al main
thread. El dispatcher corre la acción sincrónicamente en el main thread.

| Acción | Args | Efecto |
| --- | --- | --- |
| `"cd"` | `string` o `{path = string}` | Navega el panel activo. |
| `"set_focus"` / `"focus"` | `string` o `{side = string}` | Cambia foco a `"left"` o `"right"`. |
| Cualquier otra acción | per resolver | Loguea un warning si no se maneja. |

Fire-and-forget: `pairee.emit` no devuelve resultado.

---

## 3. `pairee.confirm({pos, title, body})`

```lua
local ok = pairee.confirm({
    pos   = { "center", w = 50, h = 10 },
    title = "Overwrite file?",
    body  = "The destination already exists.",
})
if not ok then return end
```

Devuelve `true` en accept, `false` en cancel.

---

## 4. `pairee.input({pos, title, value, obscure, realtime, debounce})`

```lua
local r = pairee.input({
    pos      = { "top-center", w = 60, h = 3 },
    title    = "New folder name",
    value    = "",
    obscure  = false,
    realtime = false,
    debounce = 0.3,
})
if r then print("user typed:", r.value, "event:", r.event) end
```

| Campo | Default | Efecto |
| --- | --- | --- |
| `pos` | requerido | `{"center" | "top-center", w = N, h = N}`. |
| `title` | `""` | Título del popup. |
| `value` | `""` | Texto inicial. |
| `obscure` | `false` | Enmascara el input (passwords). |
| `realtime` | `false` | Dispara con cada keystroke. |
| `debounce` | `0` | Si `realtime`, segundos a esperar antes de disparar. |

Devuelve `nil` en cancel, o `{ value, event }` en submit. `event`:

| Valor | Significado |
| --- | --- |
| `0` | desconocido / canal cerrado |
| `1` | submitted (Enter) |
| `2` | cancelled (Esc) |
| `3` | typed (solo realtime) |

---

## 5. `pairee.which({cands, silent})`

Le pide al usuario que apriete una de varias teclas. Devuelve el
índice 1-based del candidato elegido, o `nil` si cancela.

```lua
local idx = pairee.which({
    silent = false,
    cands = {
        { on = "a",                 desc = "press a" },
        { on = "<C-c>",             desc = "cancel" },
        { on = { "j", "<Down>" },   desc = "down" },
    },
})
```

`on` es un string de una sola tecla o una lista de teclas equivalentes.
`desc` es texto human-readable opcional.

> La notación de tecla sigue el mismo parser que el resolver
> (`<C-c>` = Ctrl+C, `<Down>` = flecha, etc.).

---

## 6. `pairee.notify({title, content, level, timeout})`

```lua
pairee.notify({
    title   = "Hello",
    content = "World",
    level   = "info",    -- "info" | "warn" | "error", default "info"
    timeout = 2.5,       -- auto-cerrar en segundos
})
```

El toast aparece arriba de la pantalla y desaparece después de
`timeout` (o queda hasta que el usuario haga clic, si `timeout = 0`).

---

## 7. `pairee.file_cache({file, skip})`

Devuelve una ruta absoluta bajo `<Pairee cache>/preview_cache/` que es
única para el par `(file, skip)`. Usala para cachear output caro de
previewer (conversiones de imagen, resultados de OCR, etc.).

```lua
local cache = pairee.file_cache({ file = job.file.url, skip = job.skip })
if cache then
    local f = io.open(cache, "r")
    if f then f:close() else
        -- generar el archivo de caché
    end
end
```

El directorio de caché se crea lazy en la primera llamada.
`pairee.file_cache` devuelve `nil` si el directorio no se puede crear.

---

## 8. `pairee.utils.*`

```lua
local os   = pairee.utils.target_os()      -- "linux" | "macos" | "windows" | ...
local fam  = pairee.utils.target_family()  -- "unix"  | "windows" | "wasm"
local now  = pairee.utils.time()           -- float segundos desde UNIX epoch
local hash = pairee.utils.hash("payload")  -- 16-char hex string
```

> `hash` no es criptográfico. Es estable entre corridas del mismo
> binario de Pairee pero no portable entre arquitecturas o incluso
> versiones de la library — no lo uses para comparaciones sensibles
> a seguridad.

---

## 9. `pairee.fs.*`

| Función | Efecto |
| --- | --- |
| `read(path)` → `string` o `nil, err` | Lee un archivo como texto. |
| `write(path, data)` | Escribe `data` en `path`. |
| `exists(path)` → `bool` | El path existe. |
| `stat(path)` → table o `nil, err` | Mismos campos que `stat(2)`. |
| `list(path)` → array de file entries | Lista un directorio. |
| `spawn(cmd, args, opts)` → handle | Spawnea un proceso hijo (restringido por sandbox). |
| `spawn_copy_task(src, dst, opts)` → task | Encola una copia async. |

`opts` acepta `overwrite = "ask" | "overwrite" | "skip" | "append"`,
`preserve = true | false`, y `filter = "glob"`.

> `spawn` está **solo disponible** para plugins trusted. Mirá
> [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md).

---

## 10. `pairee.ui.*`

Constructores de widgets. Devuelven un handle userdata que podés
componer en líneas y párrafos.

| Constructor | Efecto |
| --- | --- |
| `Paragraph` | Widget de texto multi-línea. |
| `Gauge` | Barra de progreso horizontal. |
| `List` | Lista vertical de items. |
| `Table` | Grilla con headers. |
| `Span`, `Line` | Fragmentos de texto inline. |

Widgets más ricos (Button, Input, Checkbox) están agendados para un
release posterior.

---

## 11. `pairee.log.*`

```lua
pairee.log.debug("loading file", { path = path })
pairee.log.info("done")
pairee.log.warn("retrying")
pairee.log.error("failed", { code = err })
```

Los mensajes van al buffer de log in-app y a `app.log` en el
directorio de caché.

---

## 12. `pairee.settings.*`

Read-only access a los settings resueltos del plugin (de `manifest.toml`
y cualquier override de nivel usuario).

```lua
local level = pairee.settings.get("log_level", "info")
```

Devuelve `nil` si la clave no está.

---

## 13. `pairee.t(key, vars)`

Lookup de string localizado con interpolación de variables.

```lua
local s = pairee.t("greeting", { name = "Lara" })
-- "greeting" = "Hello, {name}!"
```

El archivo TOML es `<plugin>/lang/<lang>.toml`. Pairee cae a `en` si
falta el idioma activo.

---

## 14. `pairee.app.*` legacy

La familia legacy sigue soportada; el código nuevo debería usar las
formas estructuradas de arriba.

| Viejo | Nuevo |
| --- | --- |
| `pairee.app.cd(path)` | `pairee.emit("cd", path)` o `pairee.emit("cd", { path = path })` |
| `pairee.app.set_focus(side)` | `pairee.emit("set_focus", side)` o `pairee.emit("focus", side)` |
| `pairee.app.confirm(title, msg)` | `pairee.confirm({pos=…, title=…, body=msg})` (loguea deprecation) |
| `pairee.app.input(title, default)` | `pairee.input({pos=…, title=…, value=default, obscure=…, realtime=…, debounce=…})` (loguea deprecation) |
| `pairee.app.notify(title, msg, level)` | `pairee.notify({title=…, content=msg, level=…, timeout=…})` |

---

## 15. Notas cross-platform

- `pairee.utils.target_os()` devuelve el string de OS compile-time de
  `std::env::consts::OS`. Usalo para gatear paths específicos de OS.
- `pairee.utils.target_family()` devuelve `"unix"`, `"windows"`, o
  `"wasm"`. Preferilo para chequeos de portabilidad.
- Las rutas de archivo en Pairee son siempre `std::path::Path`; nunca
  hardcodees `/` o `\`. Los plugins reciben paths como strings y
  deberían usar el separador apropiado de la plataforma
  (`package.config:sub(1,1)` de Lua lo da en la plataforma corriendo).

---

## A dónde ir ahora

- Sandbox y modelo de confianza: [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md)
- Autoría de plugins: https://github.com/FittyAr/Pairee/blob/master/docs/plugin-dev-guide.md
- Arquitectura de plugins: https://github.com/FittyAr/Pairee/blob/master/docs/technical/plugin-system-design.md
