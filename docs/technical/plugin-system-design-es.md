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
- El acceso de los complementos a las bibliotecas estándar peligrosas de Lua (`io`, `os`, `package`) está **bloqueado por defecto** y requiere habilitación explícita del usuario (`trusted = true`) en `pairee.toml`.
- Un parámetro de configuración global de solo lectura **`secure_mode`** evita todo acceso a internet y ejecución de comandos externos independientemente de la configuración de confianza de cada complemento.
- Oculta bajo una sección `[developer]`, una suite interna se encarga de la validación sintáctica, autoformateado, cálculo de hashes SHA-256 por cada archivo del plugin y automatización de ramas/PRs de Git.

---

## 2. Diagrama de Arquitectura

```mermaid
graph TD
    subgraph "Capa de Configuración"
        TOML[pairee.toml\ntablas [settings], [plugins] y [developer]] --> PM
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

## 5. Superposición Dinámica de Atajos de Teclado

Para facilitar el uso de los complementos, los desarrolladores pueden definir atajos de teclado directamente en el manifiesto `manifest.toml` del complemento:

```toml
# En manifest.toml del complemento
[keybindings]
"ctrl+h" = "entry"          # Vincula Ctrl+H a la función entry() del complemento
"g"      = "run_action"     # Vincula la tecla "g" al método run_action()
```

Durante el arranque, `PluginManager` lee estos atajos y los superpone automáticamente sobre el resolutor de teclado activo en Pairee, asegurando que los controles funcionen sin requerir modificaciones manuales de los archivos del proyecto por parte del usuario.

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
Para complementos avanzados que requieren integración con el sistema (como ejecutar comandos de Git o herramientas de diagnóstico), el usuario puede optar explícitamente por el **Modo Confiable** en su archivo `pairee.toml`:
* La VM se inicializa con acceso a las bibliotecas estándar peligrosas (`io`, `os`, `package`) para permitir la carga de archivos y módulos externos.
* Se permite que el complemento invoque procesos externos a través de la API `pairee.fs.spawn()`.

### 10.3 Modo Seguro Global (`secure_mode = true`)
Para evitar que incluso los complementos marcados como de confianza recopilen sigilosamente archivos y los filtren a través de la red, Pairee implementa un **Modo Seguro** global e inmutable que actúa como salvaguarda a nivel del motor en Rust:
* **Activación:** Se habilita en el archivo de configuración principal (`pairee.toml` bajo `[settings]` -> `secure_mode = true`).
* **Inmutabilidad:** El objeto de configuración principal se carga como de solo lectura por el núcleo de Rust; los scripts de Lua no tienen acceso de escritura a este espacio de memoria y no pueden desactivar el Modo Seguro.
* **Intercepción de Red y Sockets:** Incluso si un complemento está configurado con `trusted = true`, el runtime de Rust bloquea cualquier creación de sockets TCP/UDP o llamadas HTTP desde el entorno de la VM.
* **Lista Negra de Ejecución de Procesos:** El ejecutor de procesos de Rust intercepta todas las llamadas a `pairee.fs.spawn()`. Si el binario coincide con alguna herramienta de red o intérprete de comandos prohibido, la ejecución se bloquea inmediatamente:
  * **Herramientas de Red y Egreso:** `curl`, `wget`, `nc`, `netcat`, `ssh`, `scp`, `sftp`, `telnet`, `ftp`, `rsync`, `nmap`.
  * **Intérpretes de Comandos y Shells:** `sh`, `bash`, `zsh`, `csh`, `tcsh`, `powershell`, `pwsh`, `cmd`, `cmd.exe`.
  * **Runtimes e Intérpretes de Lenguajes:** `python`, `python3`, `perl`, `ruby`, `node`, `php`, `lua`, `luajit`.
* **Aislamiento del Límite del Sistema de Archivos:** Bajo el Modo Seguro, las operaciones de lectura/escritura de archivos a través de `pairee.fs` se restringen al directorio de trabajo activo y a las carpetas de configuración del usuario, impidiendo el acceso a rutas raíz o directorios del sistema.

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

### 12.2 Configuración en TUI y Renderizado Dinámico
1. **Análisis:** En la inicialización, el cargador de configuración de Pairee lee el `[settings_schema]` de todos los complementos activos.
2. **Menú de Configuración:** La pantalla de configuración TUI muestra una sección dedicada llamada "Configuración de Complementos" con subcategorías para cada plugin activo.
3. **Campos del Formulario:** Al seleccionar un complemento, se dibujan dinámicamente los controles correspondientes:
   * Los tipos `bool` se muestran como casillas de verificación (checkboxes).
   * Los tipos `string` se muestran como entradas de texto.
   * Los tipos `integer` se muestran como selectores numéricos.
   Las descripciones se muestran debajo de cada control como texto de ayuda al enfocar el elemento.

### 12.3 Persistencia de Valores (`pairee.toml`)
Todos los valores personalizados por el usuario se guardan dentro del archivo de configuración global `pairee.toml` bajo una tabla dedicada `[plugins.settings.<nombre_complemento>]`:
```toml
[plugins.settings.git-status]
show_hidden = true
git_path    = "/usr/local/bin/git"
max_depth   = 5
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

Cuando el desarrollador configura `developer_mode = true` en `pairee.toml` bajo una sección `[developer]`, se desbloquea una suite de utilidades de desarrollo:

### 14.1 Comandos de Desarrollador en CLI y Validaciones Estrictas
* `pairee developer format <ruta>`: Formatea todos los archivos Lua en el directorio del complemento usando el estándar de estilo de Pairee.
* `pairee developer validate <ruta>`: Analiza scripts con linter y valida la sintaxis Lua, ejecutando un conjunto de verificaciones estrictas de compatibilidad multiplataforma.
* `pairee developer package <ruta>`: Escanea la estructura del directorio, ejecuta el suite de validación y empaqueta el complemento:
  * **Autodetección de Idiomas:** Inspecciona la carpeta `lang/` buscando archivos `*.toml`, extrae los códigos de idioma (p. ej., `en`, `es`) y los escribe automáticamente en la lista `languages` del manifiesto `manifest.toml`.
  * **Autodetección de Categoría/Tipo:** Analiza el script `main.lua` para comprobar qué APIs o ganchos se referencian (p. ej., la presencia de funciones `peek`/`seek` clasifica el tipo como `"previewer"`; suscripciones a `pairee.ps.sub` lo clasifican como `"hook"`). Escribe esto en el campo `type` del manifiesto.
  * **Generación de Hashing de Integridad:** Genera o actualiza automáticamente el archivo `sha256.sum` con los hashes SHA-256 de cada archivo en el directorio del plugin.

### 14.2 Reglas del Suite de Validación Estricta
Los comandos `validate` y `package` (junto con el script de CI `./scripts/validate-plugin.sh`) imponen rigurosamente:
1. **Seguridad de Nombres Multiplataforma:**
   * Los nombres de archivos y directorios deben contener únicamente caracteres alfanuméricos en minúsculas, puntos (`.`), guiones (`-`) y guiones bajos (`_`).
   * Se prohíben espacios, letras mayúsculas o caracteres especiales (como `?`, `*`, `:`, `\`, `/`, `|`, `<`, `>`, `"`) incompatibles entre sistemas de archivos.
2. **Consistencia de Nombres:**
   * El nombre del directorio raíz del complemento debe coincidir exactamente con el campo `name` declarado en `manifest.toml`.
3. **Cobertura de Archivos de Localización y Ayuda:**
   * Cada idioma declarado en el array `languages` del manifiesto debe tener tanto un archivo de traducción (`lang/<locale>.toml`) como un archivo de ayuda (`help/<locale>.md`).
   * No se permite la presencia de archivos de traducción o ayuda adicionales en esos subdirectorios que no estén explícitamente listados en el array `languages` del manifiesto.
   * Deben estar presentes el archivo de traducción y de ayuda correspondientes al `default_language`.
4. **Sincronización de Claves de Traducción:**
   * Analiza todos los archivos TOML en `lang/` y verifica que el árbol de claves sea idéntico en todos ellos. Cualquier clave faltante en un idioma secundario genera una advertencia.
5. **Codificación UTF-8:**
   * Todos los archivos `.toml`, `.md` y `.lua` deben estar codificados en UTF-8 válido.

### 14.3 Asistente TUI de Metadatos y Envío Automático de PR
Se añade una pantalla interactiva TUI exclusiva para desarrolladores que actúa como asistente paso a paso de metadatos y envío de complementos:
1. **Asistente de Metadatos:** Guía al desarrollador para completar campos faltantes del manifiesto (autor, descripción, licencia), muestra los idiomas y categorías detectados automáticamente y valida el paquete.
2. **Automatización de PR:** Recopila tokens de GitHub (PATs), repositorios de forks y descripciones de commit, empaqueta el directorio (actualizando `sha256.sum`), crea una rama Git local, realiza el commit, hace push a su fork y abre automáticamente la Pull Request en GitHub.

---

## 15. Registro, Estructura de Directorios y Flujo de Comandos de la CLI

Para facilitar la distribución, el registro distribuye estructuras de directorios abiertas con comprobaciones de hash por cada archivo:

* **Estructura del Registro:** Los complementos se almacenan como carpetas sueltas en la rama de registro bajo `registry/plugins/<nombre>/<version>/`.
* **Verificación de Integridad:** Un archivo `sha256.sum` dentro de la carpeta de versión detalla el hash SHA-256 de cada archivo del complemento. El cliente descarga cada archivo individualmente y valida su hash correspondiente.
* **Comandos de Gestión de la CLI:**
  * `pairee plugin search <query>`: Búsqueda local en el catálogo de complementos. Los resultados de búsqueda muestran etiquetas coloreadas indicando la categoría (p. ej., `[Hook]`) y los idiomas soportados (p. ej., `[EN] [ES]`).
  * `pairee plugin list`: Lista los complementos instalados, configuraciones de confianza e indica si hay actualizaciones.
  * `pairee plugin check-updates`: Consulta la rama remota del registro para ver si existen nuevas versiones compatibles en SemVer.
  * `pairee plugin update`: Actualiza y valida los hashes archivo por archivo de forma automatizada.

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
