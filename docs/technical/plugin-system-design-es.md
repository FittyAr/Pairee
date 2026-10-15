# Sistema de Complementos de Pairee — Referencia Técnica de Diseño

> **Este documento describe la arquitectura Lua planificada para el sistema de complementos de Pairee. Es una especificación técnica prospectiva; todavía no existe código que implemente este sistema.**

---

## Tabla de Contenidos

1. [Descripción General](#1-descripción-general)
2. [Diagrama de Arquitectura](#2-diagrama-de-arquitectura)
3. [Estructura de Módulos](#3-estructura-de-módulos)
4. [Patrones de Diseño](#4-patrones-de-diseño)
5. [Superposición Dinámica de Atajos de Teclado](#5-superposición-dinámica-de-atajos-de-teclado)
6. [Tipos de Complemento y Hooks](#6-tipos-de-complemento-y-hooks)
7. [Referencia de la API Expuesta](#7-referencia-de-la-api-expuesta)
8. [Modelo de Concurrencia y Estado](#8-modelo-de-concurrencia-y-estado)
9. [Motor de Depuración y Registro de Bitácora](#9-motor-de-depuración-y-registro-de-bitácora)
10. [Aislamiento (Sandboxing), Modo Confiable y Protección del Modo Seguro](#10-aislamiento-sandboxing-modo-confiable-y-protección-del-modo-seguro)
11. [Localización de Complementos (I18n)](#11-localización-de-complementos-i18n)
12. [Configuración Personalizada de Complementos](#12-configuración-personalizada-de-complementos)
13. [Documentación de Ayuda Integrada F1](#13-documentación-de-ayuda-integrada-f1)
14. [Herramientas del Modo Desarrollador y TUI de Envío de Pull Request](#14-herramientas-del-modo-desarrollador-y-tui-de-envío-de-pull-request)
15. [Registro, Estructura de Directorios y Flujo de Comandos de la CLI](#15-registro-estructura-de-directorios-y-flujo-de-comandos-de-la-cli)
16. [Hitos de Implementación](#16-hitos-de-implementación)

---

## 1. Descripción General

El sistema de complementos de Pairee permitirá a los desarrolladores de la comunidad extender el comportamiento del gestor de archivos sin modificar el núcleo en Rust. Los complementos se escriben en **Lua** y son cargados al inicio por un componente `PluginManager`.

**Decisiones de diseño clave:**

- **`mlua`** (LuaJIT o Lua 5.4 compilado estáticamente) es la biblioteca de integración. Se integra nativamente con Tokio para ejecución asíncrona.
- El código de los complementos **nunca bloquea el hilo de renderizado**. Toda ejecución de complementos ocurre dentro de tareas Tokio dedicadas.
- El `AppState` de Rust **nunca es mutado directamente** por los complementos. Las mutaciones fluyen a través de una cola de eventos tipados.
- El acceso de los complementos a las bibliotecas estándar peligrosas de Lua (`io`, `os`, `package`) está **bloqueado por defecto** y requiere habilitación explícita del usuario (`trusted = true`) en `config.toml`.
- Un parámetro de configuración global de solo lectura **`secure_mode`** evita todo acceso a internet y ejecución de comandos externos independientemente de la configuración de confianza de cada complemento.
- Oculta bajo una sección `[developer]`, una suite interna se encarga de la validación sintáctica, autoformateado, cálculo de hashes SHA-256 por cada archivo del plugin y automatización de ramas/PRs de Git.

---

## 2. Diagrama de Arquitectura

```mermaid
graph TD
    subgraph "Capa de Configuración"
        TOML[config.toml\najustes y tablas [plugins.nombre]] --> PM
        LOCK[plugins.lock\nhash de archivos + versiones fijadas] --> PM
    end

    subgraph "Gestor de Complementos — src/plugin/"
        PM[PluginManager\nmod.rs] --> LOADER[loader.rs]
        PM --> REGISTRY[registry.rs]
        PM --> SANDBOX[sandbox.rs]
        PM --> DEV[developer_tool.rs]
        PM --> UPDATER[updater.rs]
    end

    subgraph "Runtime Lua — src/plugin/runtime/"
        LOADER --> LUA[instancia mlua::Lua\nstandard.rs]
        LUA --> BIND_APP[pairee.app]
        LUA --> BIND_FS[pairee.fs]
        LUA --> BIND_UI[pairee.ui]
        LUA --> BIND_PS[pairee.ps]
        LUA --> BIND_LOG[pairee.log]
        LUA --> BIND_SYNC[pairee.sync]
    end

    subgraph "Código de Complementos — ~/.config/pairee/plugins/"
        LOADER --> P1[git.pairee/main.lua]
        LOADER --> P2[fzf.pairee/main.lua]
        LOADER --> P3[custom/main.lua]
    end

    subgraph "Integración con la App"
        BIND_APP <-->|"Cola AppEvent (tokio::mpsc)"| APPSTATE[AppState]
        BIND_SYNC -->|"Canal snapshot (oneshot)"| APPSTATE
        APPSTATE --> HOOKBUS[HookBus\nhooks.rs]
        HOOKBUS --> P1
        HOOKBUS --> P2
    end

    subgraph "Capa de Renderizado Ratatui"
        APPSTATE --> DRAW[ui::mod.rs]
        BIND_UI --> DRAW
    end
```

---

## 3. Estructura de Módulos

```text
src/
└── plugin/
    ├── mod.rs              # API pública — re-exportaciones, PluginManager::init()
    ├── manager.rs          # Descubre complementos, llama al loader, configura hooks
    ├── loader.rs           # Descubre directorios, verifica archivos individuales, ejecuta main.lua
    ├── registry.rs         # Índice Nombre → PluginHandle
    ├── hooks.rs            # HookBus: subscribe/emit de eventos de ciclo de vida
    ├── sandbox.rs          # Control de acceso a stdlib de Lua por complemento y Modo Seguro
    ├── developer_tool.rs   # Formateo, validación, hashes SHA-256 y cliente PR de Git
    ├── updater.rs          # Búsqueda, descarga de archivos uno a uno y verificación de hashes
    └── runtime/
        ├── mod.rs
        ├── standard.rs     # Inicialización de la VM Lua: globals, Lua preset
        ├── bindings/
        │   ├── mod.rs
        │   ├── app.rs      # Implementación de pairee.app.*
        │   ├── fs.rs       # Implementación de pairee.fs.*
        │   ├── ui.rs       # Implementación de pairee.ui.*
        │   ├── ps.rs       # Implementación de pub/sub pairee.ps.*
        │   ├── log.rs      # Implementación de depuración pairee.log.*
        │   └── sync.rs     # Sincronización de estado
        └── types/
            ├── mod.rs
            ├── entry.rs    # Tipo FileEntry expuesto a Lua
            └── job.rs      # Tipo de contexto PreviewJob / HookEvent
```

Cada archivo en `src/plugin/` tiene una única responsabilidad, cumpliendo las reglas SRP de `AGENTS.md`.

---

## 4. Patrones de Diseño

* **Patrón Estrategia — Tipos de Complemento como Claves de Despacho:** Los complementos implementan roles específicos (`Previewer`, `Preloader`, `Hook`, `Command`) al definir los métodos correspondientes en su tabla Lua de retorno.
* **Patrón Observador — HookBus:** Un `HookBus` central mantiene el registro de suscripciones y notifica a los suscriptores de forma asíncrona mediante tareas Tokio.
* **Patrón Comando — Complementos Funcionales:** Los complementos mapean nombres a funciones `entry()` registradas en el despachador global.
* **Patrón Fachada — Globals `pairee.*`:** Las tablas `pairee.app`, `pairee.fs`, `pairee.ui` y `pairee.log` actúan como una fachada sobre los internos de Rust, exponiendo APIs limpias.
* **Patrón Flyweight / Snapshot — Lecturas de Estado:** Los complementos obtienen un snapshot de solo lectura (`pairee.sync`) y envían mutaciones a través de un canal de eventos tipados.

---

## 5. Comandos de plugins en el keymap

Los plugins declaran comandos con teclas sugeridas en `manifest.toml`
(`[[commands]]`, ver la guía de plugins §6). Cada comando se interna como
`Action::Plugin(id)` (`src/keybindings/plugin_commands.rs`) y entra al keymap
de paneles como cualquier acción propia: tiene secuencias, `<leader>`,
which-key, la paleta de comandos y overrides del usuario
(`"plugin.<nombre>.<comando>"` en `keybindings.toml`).

El loader aplica una capa de sugerencias por plugin, en orden de nombre,
después del preset y antes de los overrides del usuario. Una capa de
sugerencias nunca toma un chord: una tecla que el preset (o un plugin
anterior) ya usa, o que taparía o quedaría tapada por una secuencia, queda sin
asignar y se informa en `KeymapLoadReport::conflicts`. Cargar o descargar un
plugin levanta una marca que el bucle principal revisa para reconstruir el
keymap. `pairee.keymap.list()` / `chord_for(id)` leen una copia publicada del
keymap vivo, y `pairee.emit(id)` encola cualquier acción para que la ejecute el
bucle principal.

---

## 6. Tipos de Complemento y Hooks

### Previewers
Llamados cuando se resalta un archivo en el panel activo. El complemento renderiza contenido en el panel de vista previa.

```lua
local M = {}

function M:peek(job)
    local content = pairee.fs.read(tostring(job.file.url))
    return pairee.ui.Paragraph(content)
end

return M
```

### Lifecycle Hooks
Reaccionan a eventos de navegación de la app sin producir salida visual.

```lua
local M = {}

function M:setup(opts)
    pairee.ps.sub("on_cd", function()
        local cwd = pairee.sync(function() return pairee.app.cwd() end)
        pairee.log.info("Directorio cambiado a: " .. cwd)
    end)
end

return M
```

### Comandos Funcionales
Invocados explícitamente mediante un atajo de teclado o menú.

```lua
local M = {}

function M:entry(args)
    local result = pairee.fs.spawn("fzf", { "--height=40%" })
    if result.status == 0 and result.stdout ~= "" then
        pairee.app.cd(result.stdout:gsub("\n$", ""))
    end
end

return M
```

---

## 7. Referencia de la API Expuesta

### `pairee.app`

| Función | Devuelve | Descripción |
|---------|---------|-------------|
| `cwd()` | `string` | Directorio actual del panel activo |
| `cd(path)` | — | Navegar a `path` |
| `focus()` | `"left"\|"right"` | Panel actualmente enfocado |
| `set_focus(side)` | — | Cambiar panel enfocado |
| `notify(title, msg, level)` | — | Mostrar popup. Nivel: `"info"`, `"warn"`, `"error"` |
| `confirm(title, msg)` | `boolean` | Diálogo de confirmación bloqueante |
| `input(title, default)` | `string` | Diálogo de entrada bloqueante |
| `hovered()` | `Entry` | Entrada de archivo actualmente resaltada |

### `pairee.fs`

| Función | Devuelve | Descripción |
|---------|---------|-------------|
| `read(path)` | `string` | Leer contenido del archivo |
| `write(path, data)` | — | Escribir datos en archivo |
| `exists(path)` | `boolean` | Verificar si existe la ruta |
| `stat(path)` | `Entry` | Metadatos del archivo |
| `list(path)` | `Entry[]` | Listar entradas del directorio |
| `spawn(cmd, args)` | `Output` | Ejecutar comando externo |
| `spawn_copy_task(from, to)` | — | Copia asíncrona con barra de progreso |

### `pairee.ui`

| Constructor | Descripción |
|-------------|-------------|
| `Paragraph(text)` | Widget de bloque de texto plano |
| `Gauge(ratio, label)` | Barra de progreso |
| `List(items[])` | Lista seleccionable |
| `Table(headers[], rows[][])` | Tabla de cuadrícula |
| `Span(text, style)` | Fragmento de texto con estilo |
| `Line(spans[])` | Fila horizontal de spans |

### `pairee.ps`

| Función | Descripción |
|---------|-------------|
| `sub(event, fn)` | Suscribirse a un evento nombrado |
| `pub(event, data)` | Publicar evento a todos los suscriptores |
| `unsub(event)` | Eliminar suscripción del complemento actual |

### `pairee.log`

| Función | Descripción |
|---------|-------------|
| `info(msg)` | Registra mensaje a nivel informativo en el sistema de bitácora central |
| `warn(msg)` | Registra mensaje de advertencia |
| `error(msg)` | Registra mensaje de error |
| `debug(msg)` | Registra mensaje de depuración |

---

## 8. Modelo de Concurrencia y Estado

```
Hilo principal (bucle de renderizado a 60 fps)
    │
    ├── [tick] procesa cola AppEvent
    │       ├─ AppEvent::ShowNotification → actualiza AppState.notification
    │       ├─ AppEvent::NavigateTo(path) → actualiza AppState.active_panel.cwd
    │
    ├── [tick] HookBus::emit("on_cd", payload)
    │       └─ tokio::spawn → tarea asíncrona del complemento
    │               └─ pairee.sync(fn) → envía solicitud de snapshot
    │                       └─ hilo principal responde vía canal oneshot
    │
    └── [tick] Ratatui draw frame (lee AppState de solo lectura)
```

---

## 9. Motor de Depuración y Registro de Bitácora

Para facilitar el desarrollo de complementos, Pairee proporciona una arquitectura integrada de registro y manejo de errores:

1. **Registros Integrados:** El módulo `pairee.log` envía los mensajes directamente a la bitácora de la aplicación, escribiéndose en `~/.cache/pairee/app.log`.
2. **Aislamiento de Errores:** Si un complemento sufre una excepción (referencias nil, división por cero, etc.), esta es interceptada en la frontera de `mlua` en Rust. El bucle de la app no se ve afectado. Pairee muestra una notificación visual de error detallando la línea y el mensaje.
3. **Modo Depuración por CLI:** Los desarrolladores pueden ejecutar:
   ```bash
   pairee --plugin-debug <nombre-complemento>
   ```
   Esta bandera configura el sistema para volcar todos los errores y llamadas a `pairee.log` de ese complemento directamente en la salida estándar (stdout), permitiendo una inspección en vivo.

---

## 10. Aislamiento (Sandboxing), Modo Confiable y Protección del Modo Seguro

Pairee implementa un límite de confianza estricto y de múltiples capas para proteger el entorno del usuario frente a complementos maliciosos o con errores.

### 10.1 La Máquina Virtual de Aislamiento (Modo No Confiable por Defecto)
Cuando se ejecuta un complemento, su entorno de Lua se carga en una instancia de VM aislada. Por defecto, los complementos están restringidos al **Modo No Confiable** (`trusted = false`):

1. **Filtrado de Bibliotecas Estándar:** Solo se cargan las bibliotecas esenciales seguras en la VM:
   * **Permitidas:** `base` (excluyendo funciones peligrosas), `table`, `string`, `math`, `utf8`.
   * **Omitidas y Bloqueadas:** `io` (operaciones de archivos), `os` (entorno del sistema, comandos de shell), `package` (mecánica de resolución de módulos), `coroutine` y `debug` (introspección de la VM).
2. **Aislamiento de Funciones Globales:** Se eliminan o deshabilitan las funciones globales estándar que permiten la ejecución dinámica de código o la inclusión arbitraria de archivos:
   * **Deshabilitadas:** `load`, `loadstring`, `dofile`, `loadfile`.
   * **`require` Sobrescrito:** El cargador en Rust proporciona una implementación personalizada de la función global `require`. Esta solo permite importar submódulos Lua relativos ubicados estrictamente dentro de la carpeta instalada del propio complemento, impidiendo el acceso a scripts o módulos de todo el sistema.
3. **Restricciones de Ejecución:** Cualquier intento de llamar a `pairee.fs.spawn()` para ejecutar procesos externos genera inmediatamente un error de ejecución en el script.

### 10.2 Modo Confiable (`trusted = true`)
Para complementos avanzados que requieren integración con el sistema (como ejecutar comandos de Git o herramientas de diagnóstico), el usuario puede optar explícitamente por el **Modo Confiable** en su archivo `config.toml` (`[plugins.<nombre>] trusted = true`):
* La VM se inicializa con acceso a las bibliotecas estándar peligrosas (`io`, `os`, `package`) para permitir la carga de archivos y módulos externos.
* Se permite que el complemento invoque procesos externos a través de la API `pairee.fs.spawn()`.

### 10.3 Modo Seguro Global (`secure_mode = true`)
Para evitar que incluso los complementos marcados como de confianza recopilen sigilosamente archivos y los filtren a través de la red, Pairee implementa un **Modo Seguro** global e inmutable que actúa como salvaguarda a nivel del motor en Rust:
* **Activación:** Se habilita en el archivo de configuración principal (`secure_mode = true` en la raíz de `config.toml`).
* **Inmutabilidad:** El objeto de configuración principal se carga como de solo lectura por el núcleo de Rust; los scripts de Lua no tienen acceso de escritura a este espacio de memoria y no pueden desactivar el Modo Seguro.
* **Intercepción de Red y Sockets:** Incluso si un complemento está configurado con `trusted = true`, el runtime de Rust bloquea cualquier creación de sockets TCP/UDP o llamadas HTTP desde el entorno de la VM.
* **Lista Permitida de Procesos:** `pairee.Command` y `pairee.fs.spawn()` pasan por `plugin::command_policy::CommandPolicy`. Un programa solo se ejecuta si es un nombre simple declarado en el manifiesto (`[permissions] commands`), no es un shell / intérprete / herramienta de red / envoltorio de comandos (lista de denegación fija, que también se comprueba en el destino de un enlace simbólico) y se resuelve a través de una entrada absoluta de `PATH` (Windows: solo `.exe` / `.com`). Se ejecuta la ruta absoluta resuelta. La lista de denegación cubre también herramientas de compilación y gestores de paquetes (`cargo`, `make`, `npm`, `pip`, `go`, `gcc`, ...), contenedores, depuradores y trazadores, lanzadores (`xdg-open`, `open`, `wsl`) y herramientas que sobrescriben cualquier ruta que se les dé (`cp`, `mv`, `dd`, `tee`, `rm`, `ln`, `install`, `rsync`, `scp`, `sftp`). `Command:cwd` debe estar en la jaula de **escritura** del complemento, y las rutas que las opciones habituales de `tar`, `git` y `zip` hacen escribir (`-C`, un archivo creado, un destino de clone, ...) también están enjauladas (`command_policy::args::path_arguments`; mejor esfuerzo para esas tres herramientas). Los hijos reciben un stdin nulo salvo que se pida otra cosa, `output()` se limita a 16 MiB por flujo y tiene tiempo máximo, y los hijos se matan cuando se suelta su handle.
* **Aislamiento del Límite del Sistema de Archivos:** Bajo el Modo Seguro, las operaciones de lectura/escritura de `pairee.fs` se restringen al espacio de trabajo del complemento y a las carpetas de configuración, caché y de complementos del usuario. El espacio de trabajo es la carpeta local del panel activo (`plugin::workspace`, actualizada en cada tick), nunca la carpeta desde la que se lanzó el proceso; la carpeta personal del usuario y las raíces de sistema de archivos no son espacios de trabajo. La carpeta de configuración de Pairee es de solo lectura salvo la carpeta de datos del propio complemento, `<datos>/pairee/plugin-data/<nombre del complemento>`, que existe desde que se enlaza el runtime.
* **Recargas:** cada recarga incrementa un contador de generación y una carga iniciada antes se rechaza al registrarse (`registry::register_plugin`), de modo que una carga lenta y obsoleta no puede resucitar un complemento que el usuario acaba de desactivar.

---

---

## 11. Localización de Complementos (I18n)

Para evitar que los complementos de terceros requieran modificaciones en la base de código de localización centralizada de Pairee (`src/config/localization/`), estos empaquetan sus propias traducciones utilizando archivos TOML aislados:

### 11.1 Estructura de Archivos
Cada complemento mantiene un directorio `lang/` con archivos de idioma (nombrados según su código ISO 639-1):
```text
~/.config/pairee/plugins/git.pairee/
├── manifest.toml
└── lang/
    ├── en.toml        # Traducciones en inglés por defecto
    └── es.toml        # Traducciones en español
```

### 11.2 Motor de Resolución y Fallback
Cuando se solicita una traducción desde el complemento, el motor de localización sigue estos pasos de resolución:
1. **Locale del Usuario:** Consulta el idioma de la interfaz activo configurado por el usuario en Pairee (p. ej., `es`). Si `lang/es.toml` existe, busca la clave.
2. **Fallback del Idioma por Defecto:** Si la clave no se encuentra en el idioma del usuario, o si el archivo `es.toml` está ausente, el motor recurre a la propiedad `default_language` declarada en el manifiesto `manifest.toml` (p. ej., `lang/en.toml`).
3. **Fallback del Identificador de Clave:** Si la clave tampoco se encuentra en el archivo de fallback, se devuelve el identificador de clave sin traducir envuelto en corchetes (p. ej., `"[messages.git_error]"`). Esto evita textos vacíos en la interfaz e identifica rápidamente claves faltantes durante la ejecución.

### 11.3 Interpolación de Variables en Traducción
La función de traducción expuesta en Lua `pairee.t("clave", { var = valor })` analiza la cadena recuperada y realiza una sustitución en línea de las variables especificadas (p. ej., reemplazando `{status}` por el valor provisto en la tabla de variables).

---

## 12. Configuración Personalizada de Complementos

Pairee permite que los complementos declaren parámetros de configuración personalizados que se integran automáticamente en la interfaz de usuario de configuración de la aplicación.

### 12.1 Declaración del Esquema de Configuración (`manifest.toml`)
Los complementos declaran sus propiedades configurables bajo una tabla `[settings_schema]` en el manifiesto. Cada opción especifica su nombre, tipo (`bool`, `string` o `integer`), valor por defecto y descripción para el usuario:
```toml
[settings_schema]
show_hidden = { type = "bool", default = false, description = "Mostrar archivos ocultos de VCS" }
git_path    = { type = "string", default = "git", description = "Ruta personalizada al ejecutable Git" }
max_depth   = { type = "integer", default = 3, description = "Profundidad máxima de recursión del directorio" }
```

### 12.2 Formulario de ajustes en el Gestor de Complementos
1. **Análisis:** `runtime::settings::specs` lee el `[settings_schema]` del complemento (ordenado por clave); `pairee.settings` usa el mismo lector, así que el formulario y el complemento coinciden en tipos y valores por defecto.
2. **Apertura:** en el Gestor de Complementos (pestaña Instalados) `c` o Enter abre `PluginSettingsDialog` sobre el gestor (una línea en `dialog_list!`; estado en `app/state/popup/plugin_settings.rs`, teclas en `app/input_popup/plugin_menu/settings.rs`, dibujo en `ui/popup/plugin_menu/settings.rs`). Un complemento sin ajustes muestra un mensaje informativo.
3. **Campos:** los ajustes `bool` son casillas (Espacio); los `string`, `integer` y `number` son campos de texto. Debajo de las filas se muestran la `description` y el valor por defecto del ajuste enfocado. Botones: Guardar, Predeterminados, Cancelar; Enter sobre una fila guarda.
4. **Validación:** Guardar comprueba cada valor contra su tipo (`Kind::accepts`); uno inválido se nombra en la línea de error y recibe el foco.

### 12.3 Persistencia de Valores (`config.toml`)
Solo se guardan, como texto, los valores distintos de los del esquema, en el `config.toml` del usuario bajo `[settings.plugin_settings.<nombre_complemento>]` (`toggles::set_settings`, que guarda a través de la configuración del anfitrión y recarga los complementos para aplicar el cambio de inmediato):
```toml
[settings.plugin_settings.git-status]
show_hidden = "true"
max_depth   = "5"
```

### 12.4 Acceso desde la API Lua (`pairee.settings`)
Los valores de configuración resueltos se inyectan en el contexto de la VM de Lua como una tabla global de solo lectura llamada `pairee.settings`. El script accede directamente a las claves:
```lua
local ocultos_visibles = pairee.settings.show_hidden -- Devuelve true o false
local cmd_git = pairee.settings.git_path -- Devuelve "/usr/local/bin/git"
```

---

## 13. Documentación de Ayuda Integrada F1

Los complementos pueden proporcionar documentación de usuario que se analiza y embebe automáticamente dentro de la pantalla principal de ayuda de Pairee (F1).

### 13.1 Archivos de Ayuda Estructurados
Para mantener el espacio de trabajo del complemento organizado, los archivos de documentación de ayuda deben residir dentro de un subdirectorio dedicado `help/`, nombrados según su código de idioma ISO 639-1 (con extensión `.md`):
```text
~/.config/pairee/plugins/git.pairee/
├── manifest.toml
└── help/
    ├── en.md          # Documentación de ayuda por defecto en inglés
    └── es.md          # Documentación de ayuda en español
```

### 13.2 Renderizado de la Ayuda en la Interfaz (Tecla F1)
1. **Integración de Panel:** Al presionar `F1`, el lector de ayuda TUI muestra una disposición por pestañas: "Ayuda General" y "Ayuda de Complementos".
2. **Listado de Documentación:** Bajo "Ayuda de Complementos", el sistema lista todos los plugins activos que contienen archivos de ayuda válidos dentro de sus directorios `help/`.
3. **Formateador Markdown:** Al seleccionar un complemento, se analiza su contenido Markdown (p. ej., `help/en.md`) y se renderiza en un panel deslizable, adaptando los encabezados, listas y bloques de código al tema visual activo del TUI.

### 13.3 Resolución del Idioma del Documento
El motor de ayuda realiza la búsqueda del archivo adecuado según el idioma activo del sistema:
* Comprueba si existe `help/<locale>.md` (p. ej., `help/es.md`). Si está presente, lo carga.
* Si no está, recurre al archivo por defecto configurado bajo `default_language` en `manifest.toml` (p. ej., `help/en.md`).

---

## 14. Herramientas del Modo Desarrollador y TUI de Envío de Pull Request

Con `plugins_developer_mode = true` en `config.toml` aparece la pestaña Developer Tools en el menú de plugins; los comandos de CLI de abajo funcionan sin esa opción:

### 14.1 Comandos de Desarrollador en CLI
`pairee developer <cmd>` (`src/run/dev_cli.rs`) actúa sobre el directorio actual y no requiere el modo desarrollador:
* `init <nombre>`: Crea un complemento a partir de la rama `plugin-template`.
* `lint`: Comprueba el manifiesto y `default_language`, que exista `main.lua`, las llamadas inseguras (`os.execute`, `io.open`, `os.system`, `dofile`, `loadfile`) en un complemento sin `requires_trust` y las teclas sugeridas en cada preset.
* `package`: Ejecuta las comprobaciones de publicación (14.2), resuelve la licencia, copia el complemento a un clon local de `plugin-registry`, escribe allí `sha256.sum` y la tabla `[files]` del manifiesto (la carpeta y el manifiesto del autor nunca se modifican; un `LICENSE` MIT generado también va solo a la copia) y actualiza `registry/index.toml`. No detecta idiomas ni el tipo de complemento: `languages` y `type` se toman del manifiesto tal como están (Pairee ignora `type`).
* `submit`: `package`, luego un commit local en la rama `plugin-registry` del clon y, con un token de GitHub (escrito sin eco), push a un fork y una pull request.

### 14.2 Comprobaciones de Publicación
`validate_for_publish` (`src/plugin/developer_tool/package/validate.rs`) se detiene en el primer fallo:
1. `manifest.toml` existe y se puede analizar.
2. El `icon` del manifiesto es un archivo que se publicaría (dentro de la carpeta del complemento, no oculto, no un enlace) y una imagen legible de 256x256 o 512x512.
3. Hay al menos una captura listada; cada una es un archivo publicado y una imagen legible de al menos 640x480.
4. Existe `main.lua`, cada nombre de archivo es uno que `safe_join` del instalador acepta (sin nombres de dispositivo de Windows, puntos o espacios finales, ni nombres iguales sin distinguir mayúsculas) y se respetan los límites del registro (1000 archivos, 16 MiB por archivo, 64 MiB en total).
5. Después, antes de descargar o escribir nada, `name` y `author` solo usan `[A-Za-z0-9_-]` (lo comprueba `registry_plugin_dir`).

Al empaquetar no se validan la cobertura de traducciones, la alineación de claves, la sintaxis Lua ni la codificación UTF-8.

### 14.3 Envío de la PR
No hay asistente de metadatos: el manifiesto se toma tal como está y nada (archivos de idioma, categoría) se autodetecta. El flujo de envío (`submit.rs`):
1. **Commit:** la copia preparada del registro se confirma en la rama `plugin-registry` del clon local. Un clon con trabajo sin enviar nunca se reinicia ni se borra en un empaquetado posterior, con o sin conexión.
2. **Automatización de la PR (con token de GitHub):** el dueño del fork es la cuenta del token (no se pide URL de fork); Pairee hace fork de `FittyAr/Pairee`, espera a que el fork exista, sube a la fuerza `plugin-registry` (un solo intento de credenciales; una referencia que el servidor rechace es un error) y abre una pull request de `<usuario>:plugin-registry` a la rama oficial `plugin-registry`. Sin token imprime los comandos manuales.

---

## 15. Registro, Estructura de Directorios y Flujo de Comandos de la CLI

Para facilitar la distribución, el registro distribuye estructuras de directorios abiertas con comprobaciones de hash por cada archivo:

* **Estructura del Registro:** Los complementos se almacenan como carpetas sueltas en la rama de registro bajo `registry/plugins/<shard>/<autor>/<nombre>/` (`<shard>` es la primera letra del autor en minúscula, `_` si no es una letra). Solo se conserva la **última** versión; no hay carpeta por versión.
* **Verificación de Integridad:** El `manifest.toml` firmado de esa carpeta lleva una tabla `[files]` con el SHA-256 de cada otro archivo. El cliente descarga cada archivo individualmente, valida su hash en memoria y solo entonces prepara la instalación (`sha256.sum` se escribe para las personas; el instalador no lo usa).
* **Comandos de Gestión de la CLI** (detalle en `plugin-registry-spec.md`, sección 5):
  * `pairee plugin search <query>`: Busca en el `index.toml` remoto por nombre y descripción; los complementos bloqueados se omiten. Los resultados muestran `[Hook]` si el índice lista hooks y los idiomas soportados (p. ej., `[EN] [ES]`).
  * `pairee plugin list`: Lista los complementos instalados, configuraciones de confianza e indica si hay actualizaciones.
  * `pairee plugin check-updates`: Consulta la rama remota del registro para ver si existen versiones más nuevas (nunca una más baja).
  * `pairee plugin update`: Reinstala los complementos con una versión más nueva mediante la misma instalación comprobada y atómica (incluidas las comprobaciones de `min_pairee` / `@since`).
  * `pairee plugin remove`: Elimina la carpeta, la entrada de `plugins.lock`, la carpeta de datos y las entradas de `config.toml`.

---

## 16. Hitos de Implementación

| Hito | Entregables |
|------|-------------|
| **M1 — Base del Motor** | Dependencia `mlua`, esqueleto `src/plugin/`, `PluginManager::init()`, sandbox |
| **M2 — Bindings de API** | `pairee.app`, `pairee.fs`, `pairee.ui`, `pairee.ps`, `pairee.log` |
| **M3 — Sistema de Hooks** | `HookBus`, integración de `on_cd`, `on_hover`, `on_key` |
| **M4 — Soporte de Previewer** | Enrutamiento de `peek()` / `seek()` Lua a través de la vista previa |
| **M5 — Superposición de Teclado** | Fusión dinámica de atajos declarados en `manifest.toml` |
| **M6 — Localización de Complementos** | Carga de archivos `lang/*.toml` aislados, motor de fallbacks, bindings `pairee.t()` |
| **M7 — Configuración de Complementos** | Análisis de `[settings_schema]` del manifiesto, menú TUI de configuración, bindings de lectura |
| **M8 — Ayuda Integrada F1** | Lector de archivos `HELP.md` / `HELP.locale.md` e integración en TUI F1 |
| **M9 — Herramientas del Desarrollador** | Comandos `pairee developer` CLI, lógica de autodetección, modal TUI de asistente PR |
| **M10 — CLI del Registro** | Comandos `pairee plugin` de búsqueda/listado (con etiquetas), instalación y actualización |
| **M11 — Rama del Registro** | Rama `plugin-registry`, distribución por carpetas abiertas, index, CI |
| **M12 — Documentación Comunitaria** | Toda la documentación pública publicada |

---

*Véase también: [plugin-registry-spec.md](plugin-registry-spec.md) para el diseño de la rama del registro y el flujo de envío.*
*Véase también: [plugin-dev-guide.md](../plugin-dev-guide.md) para cómo escribir y enviar un complemento.*
