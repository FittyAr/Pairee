# Referencia de la API Lua de Plugins de Pairee

Este documento describe la superficie de la API Lua expuesta a los plugins de Pairee, incluyendo las nuevas vinculaciones introducidas en la evolución M0 del sistema de plugins. Para la arquitectura completa y la justificación de diseño, consulta [`docs/technical/plugin-system-design-es.md`](../technical/plugin-system-design-es.md) y [`docs/technical/plugin-roadmap-es.md`](../technical/plugin-roadmap-es.md). Para una guía paso a paso de cómo escribir un plugin, consulta [`docs/plugin-dev-guide-es.md`](../plugin-dev-guide-es.md).

---

## 1. Namespace global `pairee`

Dentro del `main.lua` de un plugin, la tabla `pairee` es el único punto de entrada público. Cada función de este documento es accesible como `pairee.<nombre>` (o, para la familia legacy `app.*`, como `pairee.app.<nombre>`).

| Entrada | Propósito | Estado |
|---|---|---|
| `pairee.app` | Superficie de aplicación legacy (cwd, cd, focus, set_focus, notify, confirm, input, hovered) | Estable; `confirm`/`input` emiten un aviso de deprecación — usa las formas top-level |
| `pairee.emit(action, args)` | Despacha cualquier acción registrada por nombre | Nuevo en M0 |
| `pairee.confirm({pos, title, body})` | Abre un diálogo de confirmación real (Sí/No) | Nuevo en M0 (la UI del popup llega en M1) |
| `pairee.input({pos, title, value, obscure, realtime, debounce})` | Abre un diálogo de entrada real | Nuevo en M0 (la UI del popup llega en M1) |
| `pairee.which({cands, silent})` | Pide al usuario presionar una de varias teclas candidatas | Nuevo en M0 (la UI del popup llega en M1) |
| `pairee.notify({title, content, level, timeout})` | Muestra una notificación estructurada | Nuevo en M0 |
| `pairee.file_cache({file, skip})` | Obtiene una ruta de caché estable para un par `(archivo, skip)` | Nuevo en M0 |
| `pairee.utils.target_os()` | Devuelve `"linux"` / `"macos"` / `"windows"` / ... | Nuevo en M0 |
| `pairee.utils.target_family()` | Devuelve `"unix"` / `"windows"` / `"wasm"` | Nuevo en M0 |
| `pairee.utils.time()` | Devuelve el epoch UNIX actual en segundos (float) | Nuevo en M0 |
| `pairee.utils.hash(str)` | Devuelve un hash estable de 64 bits de `str` como cadena hex | Nuevo en M0 |
| `pairee.fs.*` | Operaciones de filesystem (`read`, `write`, `exists`, `stat`, `list`, `spawn`, `spawn_copy_task`) | Estable |
| `pairee.ui.*` | Constructores de widgets (`Paragraph`, `Gauge`, `List`, `Table`, `Span`, `Line`) | Estable; los widgets userdata más ricos llegan en M4 |
| `pairee.ps.sub / pub / unsub` | Pub/sub local | Estable |
| `pairee.log.*` | Loguea un mensaje en el nivel dado | Estable |
| `pairee.sync(fn)` | Bridge de snapshot al estado del hilo principal | Estable; el path dual sync/async completo llega en M3 |
| `pairee.settings.*` | Acceso de lectura a los settings resueltos del plugin | Estable |
| `pairee.t(key, vars)` | Búsqueda de cadenas localizadas con interpolación de variables | Estable |
| `pairee._secure_mode` | Booleano: `true` cuando Secure Mode global está activo | Estable |

---

## 2. `pairee.emit(action, args)` — despacho de acciones

`pairee.emit` es el punto de entrada unificado para disparar cualquier acción registrada en el resolver de keybindings de la aplicación.

```lua
pairee.emit("cd", "/tmp")                        -- argumento string
pairee.emit("cd", { path = "/tmp" })             -- argumento tabla
pairee.emit("set_focus", "left")                 -- alias: "focus" también funciona
pairee.emit("select", { url = f.url, state = true })
```

`args` se convierte a un valor JSON (tabla Lua → objeto JSON, tabla Lua con índices enteros → array JSON, escalar → escalar JSON) y se reenvía al hilo principal. El despachador ejecuta la acción sincrónicamente en el hilo principal.

| Estado | Nombre de acción | Args | Efecto |
|---|---|---|---|
| M0 | `"cd"` | `string` o `{path = string}` | Navega el panel activo a la ruta dada |
| M0 | `"set_focus"` / `"focus"` | `string` o `{side = string}` | Cambia el foco a `"left"` o `"right"` |
| Futuro | *cualquier otra acción* | según resolver | Loguea un aviso en M0; despachará a través del resolver en una fase posterior |

El despachador es fire-and-forget; `pairee.emit` no devuelve resultado.

---

## 3. `pairee.confirm({pos, title, body})` y `pairee.input({pos, title, value, obscure, realtime, debounce})`

Estas son las nuevas APIs estructuradas de diálogo. Reemplazan los stubs legacy `pairee.app.confirm(title, msg)` y `pairee.app.input(title, default)`.

```lua
-- Confirm: devuelve true si el usuario acepta, false si cancela.
local ok = pairee.confirm({
    pos   = { "center", w = 50, h = 10 },
    title = "¿Sobrescribir archivo?",
    body  = "El archivo de destino ya existe.",
})
if not ok then return end

-- Input: devuelve una tabla { value, event } al enviar, o nil al cancelar.
local resultado = pairee.input({
    pos      = { "top-center", w = 60, h = 3 },
    title    = "Nuevo nombre de carpeta",
    value    = "",
    obscure  = false,
    realtime = false,
    debounce = 0.3,
})
if resultado then
    print("usuario escribió:", resultado.value, "event:", resultado.event)
end
```

`event` es una etiqueta entera:

| Valor | Significado |
|---|---|
| 0 | desconocido / canal cerrado (default) |
| 1 | enviado (Enter) |
| 2 | cancelado (Esc) |
| 3 | tecleado (solo realtime) |

**Nota M0**: el despachador enruta la solicitud, pero el cableado real del popup TUI llega en M1. En M0 ambos diálogos devuelven valores placeholder (`false` para confirm, `submitted` con el valor por defecto para input) para que los plugins que migren temprano obtengan una respuesta determinista.

### 3.1 Legacy `pairee.app.confirm(title, msg)` y `pairee.app.input(title, default)`

Siguen funcionando pero loguean un aviso de deprecación. Migra a las formas estructuradas anteriores.

---

## 4. `pairee.which({cands, silent})` — prompt de teclas

Pide al usuario presionar una de las teclas candidatas, devuelve el índice 1-based del candidato seleccionado (o `nil` si el usuario cancela).

```lua
local idx = pairee.which({
    silent = false,
    cands = {
        { on = "a",                 desc = "presiona a" },
        { on = "<C-c>",             desc = "cancelar" },
        { on = { "j", "<Down>" },   desc = "abajo" },
    },
})
if idx == 1 then
    -- el usuario presionó a
end
```

`on` puede ser una sola cadena de tecla o una lista de teclas equivalentes. `desc` es una descripción legible opcional que se muestra junto al candidato.

**Nota M0**: el cableado real del popup TUI llega en M1. En M0 el despachador devuelve `nil` (cancelar) para que los plugins que migren temprano obtengan un placeholder determinista.

---

## 5. `pairee.notify({title, content, level, timeout})` — notificación estructurada

```lua
pairee.notify({
    title   = "Hola",
    content = "Mundo",
    level   = "warn",        -- "info" | "warn" | "error", default "info"
    timeout = 2.5,          -- auto-cerrar en segundos (M0: logueado pero no aplicado)
})
```

La forma legacy `pairee.app.notify(title, msg, level)` sigue funcionando. Los plugins nuevos deberían usar la forma estructurada.

---

## 6. `pairee.file_cache({file, skip})` — ruta de caché estable

Devuelve una ruta absoluta bajo `<caché de Pairee>/preview_cache/` que es única al par `(archivo, skip)`. Úsala para cachear salida costosa del previewer (conversiones de imagen, resultados de OCR, etc.) entre invocaciones.

```lua
local cache = pairee.file_cache({ file = job.file.url, skip = job.skip })
if cache then
    local f = io.open(cache, "r")
    if f then f:close() else -- genera el archivo de caché end
end
```

El directorio de caché se crea perezosamente en la primera llamada. `pairee.file_cache` devuelve `nil` si el directorio de caché no puede crearse.

---

## 7. `pairee.utils.*` — utilidades básicas

```lua
local so   = pairee.utils.target_os()      -- "linux" | "macos" | "windows" | ...
local fam  = pairee.utils.target_family()  -- "unix"  | "windows" | "wasm"
local now  = pairee.utils.time()           -- float segundos desde el epoch UNIX
local hash = pairee.utils.hash("payload")  -- string hex de 16 caracteres
```

`hash` no es criptográfico (usa el `DefaultHasher` de la librería estándar, basado en xxhash). Es estable entre ejecuciones del mismo binario de Pairee pero no es portable entre arquitecturas ni versiones de librería — no lo uses para comparaciones sensibles a la seguridad.

---

## 8. Sandbox y secure mode

Los plugins corren en un sandbox Lua que:

- Expone solo `base`, `table`, `string`, `utf8` y `math` (en modo untrusted).
- Elimina `load`, `loadstring`, `dofile` y `loadfile` (en modo untrusted).
- Reemplaza `require` con un loader limitado al directorio del propio plugin.
- Bloquea `pairee.fs.spawn` y cualquier acceso a `io`/`os`/`package` (en modo untrusted).

Cuando Secure Mode está activo en el `pairee.toml` del usuario:

- `pairee.fs.spawn` se blacklistea adicionalmente contra una lista de 27 comandos (herramientas de red, shells, runtimes de scripts).
- Las operaciones de filesystem se restringen al workspace activo + los directorios de config y caché del usuario.

Consulta la sección `[sandbox]` en `docs/plugin-dev-guide-es.md` para la matriz completa.

---

## 9. Notas cross-platform

- `pairee.utils.target_os()` devuelve la cadena del SO en tiempo de compilación desde `std::env::consts::OS`. Úsala para gatear paths de código específicos del SO.
- `pairee.utils.target_family()` devuelve `"unix"`, `"windows"` o `"wasm"`. Prefiér sobre `target_os` para chequeos de portabilidad.
- Las rutas de archivo en Pairee son siempre `std::path::Path`; nunca hardcodees `/` ni `\`. Los plugins reciben rutas como strings y deberían usar el separador apropiado de la plataforma (`package.config:sub(1,1)` en Lua lo da en la plataforma en ejecución).

---

## 10. Cheatsheet de migración

| Antiguo (M0 y anteriores) | Nuevo (M0+) | Notas |
|---|---|---|
| `pairee.app.cd(path)` | `pairee.emit("cd", path)` o `pairee.emit("cd", { path = path })` | La forma antigua sigue funcionando |
| `pairee.app.set_focus(side)` | `pairee.emit("set_focus", side)` o `pairee.emit("focus", side)` | La forma antigua sigue funcionando |
| `pairee.app.confirm(title, msg)` | `pairee.confirm({pos=..., title=title, body=msg})` | La forma antigua loguea deprecación, devuelve `true` |
| `pairee.app.input(title, default)` | `pairee.input({pos=..., title=title, value=default, obscure=..., realtime=..., debounce=...})` | La forma antigua loguea deprecación, devuelve `default` |
| `pairee.app.notify(title, msg, level)` | `pairee.notify({title=title, content=msg, level=level, timeout=...})` | La forma antigua sigue funcionando |
| (sin equivalente) | `pairee.which({cands=..., silent=...})` | M0 devuelve `nil` (cancelar); M1 cablea el popup |
| (sin equivalente) | `pairee.file_cache({file=..., skip=...})` | M0 totalmente funcional |
| (sin equivalente) | `pairee.utils.target_os / target_family / time / hash` | M0 totalmente funcional |

Para el análisis de brechas completo y el roadmap a M1–M5, consulta [`docs/technical/plugin-roadmap-es.md`](../technical/plugin-roadmap-es.md).
