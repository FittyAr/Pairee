# Guía para Desarrolladores de Complementos de Pairee

> **Esta guía explica cómo escribir, probar y enviar un complemento Lua para Pairee.**

---

## Tabla de Contenidos

1. [Descripción General](#1-descripción-general)
2. [Tipos de Complemento](#2-tipos-de-complemento)
3. [Tu Primer Complemento — Hola Mundo](#3-tu-primer-complemento--hola-mundo)
4. [Estructura de Archivos del Complemento](#4-estructura-de-archivos-del-complemento)
5. [Resumen de la API](#5-resumen-de-la-api)
6. [Superposición de Atajos Dinámicos](#6-superposición-de-atajos-dinámicos)
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

Los complementos se almacenan en tu carpeta de configuración y se declaran en `pairee.toml`. Una vez cargados, se ejecutan de manera asíncrona en hilos de segundo plano sin bloquear el renderizado de la interfaz de terminal.

---

## 2. Tipos de Complemento

| Tipo | Métodos Lua | Invocado Cuando |
|------|-------------|-----------------|
| **Previewer** | `peek(job)`, `seek(job)` | Un archivo es resaltado en el panel activo |
| **Preloader** | `preload(job)` | Un archivo está a punto de entrar en vista |
| **Hook** | `setup(opts)`, suscripciones via `pairee.ps.sub` | Se dispara un evento de ciclo de vida (p.ej. `on_cd`) |
| **Comando** | `entry(args)` | Un atajo de teclado o entrada de menú llama al complemento por nombre |
| **Mixto** | Combinación de los anteriores | Múltiples roles |

---

## 3. Tu Primer Complemento — Hola Mundo

Crea la carpeta `~/.config/pairee/plugins/hello/`.

Crea el archivo `~/.config/pairee/plugins/hello/main.lua`:

```lua
local M = {}

function M:entry()
    pairee.app.notify("¡Hola!", "Este es mi primer complemento de Pairee.", "info")
end

return M
```

Crea el archivo de manifiesto `~/.config/pairee/plugins/hello/manifest.toml`:

```toml
name = "hello"
version = "1.0.0"
description = "Complemento simple de prueba Hola Mundo"
author = "tu-usuario"
license = "MIT"
type = "command"
min_pairee = "0.7.0"
```

Regístralo en `pairee.toml`:

```toml
[plugins.hello]
name    = "hello"
trusted = false
```

Lanza Pairee, abre la entrada de comandos (o vincúlalo a una tecla) y ejecuta `plugin:hello`. Aparecerá una notificación.

---

## 4. Estructura de Archivos del Complemento

Un complemento es una carpeta que contiene un script Lua principal, un manifiesto y submódulos o recursos opcionales:

```text
~/.config/pairee/plugins/
└── my-plugin/
    ├── main.lua              # Requerido — punto de entrada
    ├── manifest.toml         # Requerido — metadatos y atajos de teclado
    ├── utils.lua             # Submódulo opcional
    └── locale/               # Archivos de localización opcionales
        ├── en.toml
        └── es.toml
```

`main.lua` **debe** devolver una tabla Lua `M` que contenga las funciones de ciclo de vida/activación.

---

## 5. Resumen de la API

### `pairee.app` — Control de la Aplicación

```lua
pairee.app.cwd()                        -- string: directorio actual
pairee.app.cd(path)                     -- navegar a path
pairee.app.focus()                      -- "left" | "right"
pairee.app.set_focus(side)              -- cambiar panel
pairee.app.notify(title, msg, level)   -- mostrar popup ("info","warn","error")
pairee.app.confirm(title, msg)          -- boolean: diálogo de confirmación
pairee.app.input(title, default)        -- string: diálogo de entrada de texto
pairee.app.hovered()                    -- Entry: archivo actualmente resaltado
```

### `pairee.fs` — Sistema de Archivos y Procesos

```lua
pairee.fs.read(path)                    -- string: contenido del archivo
pairee.fs.write(path, data)            -- escribe datos en archivo
pairee.fs.exists(path)                 -- boolean
pairee.fs.stat(path)                   -- Entry: metadatos del archivo
pairee.fs.list(path)                   -- Entry[]: listado del directorio
pairee.fs.spawn(cmd, args)             -- Output: {stdout, stderr, status}
pairee.fs.spawn_copy_task(from, to)    -- copia en segundo plano con barra de progreso
```

### `pairee.ui` — Constructores de Widgets

```lua
pairee.ui.Paragraph(text)
pairee.ui.Gauge(ratio, label)           -- ratio: 0.0 a 1.0
pairee.ui.List(items)                   -- items: string[]
pairee.ui.Table(headers, rows)
pairee.ui.Span(text, style)
pairee.ui.Line(spans)
```

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

## 6. Superposición de Atajos Dinámicos

En lugar de requerir que los usuarios modifiquen manualmente sus archivos de configuración global, un complemento puede declarar sus atajos predeterminados directamente dentro de su `manifest.toml`:

```toml
# En manifest.toml
[keybindings]
"ctrl+h" = "entry"          # Vincula Ctrl+H a la función entry() de este complemento
"g"      = "run_action"     # Mapea la tecla "g" a la función run_action
```

Cuando se carga el complemento, el resolutor de atajos de Pairee fusiona automáticamente estos accesos directos en el entorno de ejecución. Si el usuario desinstala el complemento, los atajos se eliminan de forma limpia.

---

## 7. Escribir un Complemento Previewer

Los previewers implementan `peek(job)` y opcionalmente `seek(job)` para desplazamiento.

El parámetro `job` provee:
- `job.file` — la `Entry` resaltada (con `.url`, `.mime`, `.size`, etc.)
- `job.area` — dimensiones del área de vista previa disponible
- `job.skip` — desplazamiento de scroll actual

```lua
local M = {}

function M:peek(job)
    if not job.file.url:match("%.csv$") then
        return  -- no es un archivo CSV, declinar
    end

    local content = pairee.fs.read(tostring(job.file.url))
    local rows = {}
    for line in content:gmatch("[^\n]+") do
        local cols = {}
        for col in line:gmatch("[^,]+") do
            cols[#cols + 1] = col
        end
        rows[#rows + 1] = cols
    end

    local headers = table.remove(rows, 1)
    return pairee.ui.Table(headers, rows)
end

return M
```

Registrar como previewer para archivos CSV en `pairee.toml`:

```toml
[[previewers]]
mime = "text/csv"
plugin = "csv-preview"
```

---

## 8. Escribir un Complemento Hook

Los complementos hook se suscriben a eventos de ciclo de vida durante su llamada `setup(opts)`.

**Eventos disponibles:**

| Evento | Payload | Se dispara Cuando |
|--------|---------|-----------------|
| `on_cd` | `{ cwd: string }` | El panel activo cambia de directorio |
| `on_hover` | `{ entry: Entry }` | El usuario mueve el cursor a un archivo diferente |
| `on_key` | `{ key: string }` | Se presiona una tecla (antes del resolver) |
| `on_focus` | `{ side: string }` | El foco de panel cambia |

```lua
local M = {}

function M:setup(opts)
    pairee.ps.sub("on_cd", function(payload)
        pairee.log.info("Navegado a: " .. payload.cwd)
        local result = pairee.fs.spawn("git", { "-C", payload.cwd, "status", "--short" })
        if result.status == 0 and result.stdout ~= "" then
            pairee.app.notify("Git Status", result.stdout, "info")
        end
    end)
end

return M
```

---

## 9. Escribir un Complemento Comando

Los complementos comando implementan `entry(args)`. Son invocados explícitamente mediante un atajo de teclado o comando usando `plugin:<nombre>`.

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
* **Aislamiento de Require:** La función global `require` es una implementación personalizada en Rust que restringe la carga de módulos exclusivamente a archivos dentro del directorio del propio complemento.
* **Sin Comandos Externos:** Cualquier llamada a `pairee.fs.spawn()` generará inmediatamente un error de ejecución.

```toml
# En pairee.toml
[plugins.csv-preview]
name    = "csv-preview"
trusted = false       # No requiere acceso al sistema (modo sandbox no confiable)

[plugins.git-status]
name    = "git-status"
trusted = true        # Solicita permisos para ejecutar herramientas externas (modo confiable)
```

### 13.2 Modo Confiable (`trusted = true`)
Cuando un complemento es marcado explícitamente como confiable por el usuario, se ejecuta en **Modo Confiable**:
* El complemento puede utilizar la gestión de archivos y módulos estándar de Lua (`io`, `os`, `package`).
* El complemento puede ejecutar comandos del sistema a través de la API `pairee.fs.spawn()`.

### 13.3 Modo Seguro Global (`secure_mode = true`)
Para evitar la filtración de datos, los usuarios pueden habilitar un **Modo Seguro** global en su configuración principal. Este modo actúa como un cortafuegos a nivel de motor:
* **Activación:** Se configura en `pairee.toml` bajo `[settings]` -> `secure_mode = true`. Esta configuración es de solo lectura en tiempo de ejecución y completamente inmutable desde Lua.
* **Prohibición de Red y Sockets:** La creación de sockets TCP/UDP y las solicitudes HTTP están completamente deshabilitadas a nivel de motor para todos los complementos, incluso aquellos con `trusted = true`.
* **Lista Negra de Ejecución de Procesos:** El motor de ejecución de procesos bloquea los intentos de ejecutar cualquier comando que coincida con utilidades de red, shells o entornos de scripting:
  * **Utilidades de Red:** `curl`, `wget`, `nc`, `netcat`, `ssh`, `scp`, `sftp`, `telnet`, `ftp`, `rsync`, `nmap`.
  * **Shells e Intérpretes:** `sh`, `bash`, `zsh`, `csh`, `tcsh`, `powershell`, `pwsh`, `cmd`, `cmd.exe`.
  * **Runtimes de Scripts:** `python`, `python3`, `perl`, `ruby`, `node`, `php`, `lua`, `luajit`.
* **Sandboxing del Sistema de Archivos:** Las API de archivos (`pairee.fs`) están restringidas al espacio de trabajo activo y a la carpeta de configuración del usuario.

---

## 14. Probar tu Complemento Localmente

1. **Coloca la carpeta de tu complemento** bajo `~/.config/pairee/plugins/<nombre>/`.
2. **Regístralo** en `pairee.toml`.
3. Ejecuta Pairee y revisa los registros en `~/.cache/pairee/app.log` para ver los eventos de ejecución.

---

## 15. Herramientas del Modo Desarrollador y TUI de Envío de PR

Para simplificar el formateado y la validación de los complementos, los desarrolladores pueden activar una suite de desarrollo en `pairee.toml`:

```toml
# En pairee.toml
[developer]
developer_mode = true   # Habilita el acceso a los comandos de CLI y pantallas de PR
```

### 15.1 Comandos de Desarrollador en CLI y Autodetección
Con el modo desarrollador activo, se desbloquean los siguientes comandos:
* `pairee developer format <ruta>`: Formatea todos los archivos Lua en el directorio del complemento usando el estándar de Pairee.
* `pairee developer validate <ruta>`: Analiza scripts con linter, verifica el esquema de `manifest.toml`, valida la sintaxis Lua y comprueba que se cumplan estrictamente las reglas de nomenclatura de archivos, consistencia de manifiesto, cobertura de traducciones y ayuda, sincronización de claves de traducción y codificación de texto.
* `pairee developer package <ruta>`: Escanea la estructura del directorio del complemento, ejecuta el suite de validación y empaqueta el complemento:
  * **Autodetecta Idiomas:** Escanea el directorio `lang/` buscando archivos TOML y los registra en el manifiesto.
  * **Autodetecta Categoría del Complemento:** Analiza las referencias en `main.lua` para identificar el tipo de plugin.
  * **Genera Hashes:** Genera/actualiza automáticamente el archivo de hashes de integridad `sha256.sum` para todos los archivos de la carpeta. (Nota: esto no crea un archivo comprimido; los complementos en el registro de Pairee se distribuyen como carpetas abiertas).
  * **Detección y Generación de Licencia:**
    - Busca un archivo de licencia `LICENSE` (sin distinguir mayúsculas/minúsculas) en la raíz del plugin.
    - Si el archivo existe pero el campo `license` en `manifest.toml` está vacío, el asistente solicita ingresar el nombre de la licencia (o asigna `"Custom"` en entornos no interactivos).
    - Si no existe ningún archivo de licencia, asigna automáticamente la licencia `"MIT"` en el manifiesto y genera un archivo `LICENSE` con la licencia MIT estándar y los datos de derechos de autor en el espacio de trabajo del plugin.

### 15.2 Herramientas de Desarrollo en TUI y Asistente de PR
Las herramientas interactivas de desarrollo en TUI (accesibles a través de la pestaña `Developer Tools` de `F11` cuando `developer_mode = true` está habilitado en `pairee.toml`) ofrecen el siguiente conjunto de utilidades:

* **Seleccionar Plugin Activo (Opción 0):** Selecciona el complemento en desarrollo sobre el cual se realizarán las tareas de linting, empaquetado e instalación local. Enumera todos los plugins bajo `plugins_dev_dir`. Adicionalmente, escanea los directorios activos del Panel 1 y del Panel 2; si alguno contiene un archivo `manifest.toml`, lo añade como opción seleccionable. Puedes seleccionarlo ingresando su nombre de carpeta, su ruta absoluta, o usando los alias `panel1`/`panel2` (o `left`/`right`).
* **Inicializar plantilla (Opción 1):** Guía paso a paso al desarrollador para crear un nuevo esqueleto de plugin en el directorio de desarrollo. Al completarse con éxito, el nuevo plugin se selecciona automáticamente en la Opción 0 y esta opción queda deshabilitada.
* **Verificar (Lint) (Opción 2):** Escanea la carpeta del plugin activo y ejecuta comprobaciones de sintaxis y seguridad tanto en los archivos Lua como en el manifiesto.
* **Empaquetar (Opción 3):** Prepara un clon local temporal de la rama `plugin-registry`, copia todos los recursos del plugin, construye/actualiza las entradas de catálogo en el archivo maestro de índice y muestra en pantalla la ruta absoluta exacta de la caché local donde se empaquetó.
* **Instalar localmente (Opción 4):** Copia el plugin activo directamente al directorio local de plugins de Pairee y lo registra en `plugins.lock` para poder probarlo de forma inmediata en la interfaz.
* **Enviar plugin (GitHub PR) (Opción 5):** Asistente que recopila descripciones, realiza commits git locales y opcionalmente utiliza un token PAT de GitHub para subir los cambios a un fork y abrir una Pull Request de forma automatizada.

### 15.3 Guía de Resolución de Problemas de Validación
Al ejecutar `pairee developer validate <ruta>` (o durante la verificación del CI), podrías encontrarte con los siguientes errores de validación. A continuación te explicamos cómo corregirlos:

| Código de Error / Advertencia | Causa Raíz | Solución |
|-------------------------------|------------|----------|
| `ERR_INVALID_NAME_CHAR` | El nombre de un archivo o carpeta contiene espacios, mayúsculas o caracteres especiales. | Cambia el nombre de los archivos/directorios para usar solo caracteres alfanuméricos en minúsculas, puntos, guiones y guiones bajos. |
| `ERR_MANIFEST_NAME_MISMATCH` | El nombre de la carpeta raíz del complemento no coincide con `name` en `manifest.toml`. | Cambia el nombre de la carpeta raíz para que coincida exactamente con el especificado en el manifiesto. |
| `ERR_LANG_FILE_MISSING` | Un idioma declarado en `languages` no tiene su archivo `lang/<locale>.toml`. | Crea el archivo `.toml` correspondiente en la carpeta `lang/` o elimina el idioma del array en el manifiesto. |
| `ERR_HELP_FILE_MISSING` | Un idioma declarado en `languages` no tiene su archivo `help/<locale>.md`. | Crea el archivo de ayuda `.md` correspondiente en la carpeta `help/`. |
| `ERR_DEFAULT_LANG_MISSING` | El idioma por defecto `default_language` no cuenta con sus archivos de traducción/ayuda. | Asegúrate de que existan tanto `lang/<default_lang>.toml` como `help/<default_lang>.md`. |
| `WARN_KEY_MISALIGNMENT` | Falta alguna clave de traducción en un archivo de idioma que sí está en otro. | Actualiza todos los archivos de `lang/*.toml` para que tengan exactamente las mismas claves de traducción. |
| `ERR_INVALID_ENCODING` | Un archivo de texto no está codificado en UTF-8. | Vuelve a guardar el archivo utilizando la codificación UTF-8 en tu editor de texto. |

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
├── manifest.toml          # Bloque [plugin] con tokens PLUGIN_NAME, PLUGIN_DESCRIPTION, PLUGIN_AUTHOR
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

1. **Activar el Modo Desarrollador** en `pairee.toml`.
2. Abrir la pantalla del **Asistente de Metadatos TUI** o ejecutar `pairee developer package ~/.config/pairee/plugins/mi-complemento` para validar tus archivos, copiarlos a una copia de trabajo local de la rama `plugin-registry` y actualizar el archivo de catálogo central `registry/index.toml`.
3. Asegurarse de que el plugin cumpla con los siguientes requisitos obligatorios de publicación:
   - **Icono:** Debe existir un icono PNG llamado `icon.png` en el directorio raíz del plugin (tamaños recomendados: `512x512` o `256x256` píxeles).
   - **Capturas de pantalla:** Debe existir un directorio `screenshots/` en la raíz del plugin que contenga al menos una captura de pantalla (PNG, JPG o JPEG).
4. Ejecutar la opción de empaquetado para clonar y preparar la rama `plugin-registry` localmente, verificar todas las firmas, copiar los archivos de código y generar la suma de verificación y la entrada del índice central.
5. Ejecutar la opción "Enviar". El asistente te solicitará:
   - **Descripción del Commit:** Una descripción detallada de los cambios o características.
   - **Token de GitHub (Opcional):** Si se suministra, Pairee realizará automáticamente un fork del repositorio, subirá la rama y creará una Pull Request. Si se omite, Pairee realizará un commit git local e imprimirá los comandos manuales para que los copies y ejecutes para subir la rama y abrir la Pull Request por ti mismo.

---

## 17. Localización de tu Complemento (Traducción)

Los complementos de Pairee admiten traducciones aisladas y autocontenidas. No es necesario modificar los archivos de localización principales de la aplicación.

### 17.1 Creación de Archivos de Idioma
Crea un subdirectorio `lang/` en la carpeta raíz de tu complemento y coloca allí archivos de traducción TOML nombrados con su código de idioma ISO-639-1:
```text
~/.config/pairee/plugins/mi-complemento/
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

Los complementos pueden declarar sus propios parámetros de configuración. Estos se cargan y renderizan dinámicamente en la ventana de opciones principal TUI de Pairee, lo que permite a los usuarios modificar variables sin tener que editar directamente el código de Lua.

### 18.1 Definición de settings_schema
En el manifiesto `manifest.toml` de tu complemento, define una tabla `[settings_schema]` que contenga las opciones configurables. Cada opción debe especificar un tipo `type` (`bool`, `string` o `integer`), un valor por defecto `default` y una breve descripción `description`:
```toml
[settings_schema]
show_hidden = { type = "bool", default = false, description = "Mostrar archivos ocultos de VCS" }
git_path    = { type = "string", default = "git", description = "Ruta personalizada al ejecutable Git" }
max_depth   = { type = "integer", default = 3, description = "Profundidad máxima de recursión" }
```

Cuando los usuarios naveguen a la ventana de opciones, Pairee presentará un formulario específico para tu complemento con estos controles. Las opciones elegidas por el usuario se guardan de forma persistente en su archivo `pairee.toml`.

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
~/.config/pairee/plugins/mi-complemento/
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

[keybindings]
"ctrl+h" = "entry"
```

---

## 21. Mejores Prácticas y Convenciones

| Convención | Guía |
|------------|------|
| **Nombres de complementos** | Usa el estilo `<name>.pairee` para complementos del registro (p.ej. `git.pairee`, `fzf.pairee`) |
| **Eventos con espacio de nombres** | Prefija los eventos pub/sub con el nombre del complemento: `mi-complemento:evento` |
| **Declinar con gracia** | En `peek()`, verifica el tipo de archivo y retorna anticipadamente si no aplica |
| **Asíncrono primero** | Evita llamadas bloqueantes. Usa `pairee.fs.spawn` y patrones asíncronos |
| **Notificaciones de error** | Muestra `pairee.app.notify` con nivel `"error"` en fallos — nunca falles silenciosamente |
| **Transparencia de confianza** | Si tu complemento necesita `trusted = true`, documéntalo claramente en el manifiesto |
| **Disciplina SemVer** | Incrementa MAJOR solo en cambios de comportamiento que rompen compatibilidad |
| **Superficie mínima** | Expón solo lo que el complemento necesita — menos superficie = menos vector de ataque |

---

*Véase también: [docs/technical/plugin-system-design-es.md](technical/plugin-system-design-es.md) para la arquitectura del motor Rust.*
*Véase también: [docs/technical/plugin-registry-spec.md](technical/plugin-registry-spec.md) para el esquema del registro y el flujo de envío.*
