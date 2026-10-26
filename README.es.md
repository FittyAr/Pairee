# Pairee

> Tu mundo, en dos paneles.

[![Licencia: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.96%2B-orange.svg)](https://www.rust-lang.org)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/FittyAr/Pairee)
![CodeRabbit Pull Request Reviews](https://img.shields.io/coderabbit/prs/github/FittyAr/Pairee?utm_source=oss&utm_medium=github&utm_campaign=FittyAr%2FPairee&labelColor=171717&color=FF570A&link=https%3A%2F%2Fcoderabbit.ai&label=CodeRabbit+Reviews)

## Pairee - ⚡ Gestor de Archivos de Terminal Moderno de Doble Panel

Pairee es un gestor de archivos para terminal moderno, multiplataforma y altamente modular, inspirado en los clásicos **Norton Commander** y **Far Manager**. Desarrollado en Rust utilizando `ratatui` y `crossterm`, su objetivo es proporcionar una experiencia de gestión de archivos eficiente, rápida y extensible directamente en tu consola.

- 🚀 **Soporte Asíncrono Completo**: Todas las operaciones pesadas del sistema de archivos (Copiar, Mover, Borrado Seguro, Eliminar) se procesan en segundo plano (`tokio`), manteniendo la interfaz perfectamente fluida y responsiva.
- 💪 **Planificación de Tareas Asíncronas**: Seguimiento del progreso en tiempo real con ventanas emergentes de barras de progreso y cancelación de tareas para trabajadores concurrentes.
- 🎨 **Temas Visuales y Diseños**: Cargador de temas visuales personalizados (como Slate, Blue, entre otros) y distribución adaptable del diseño de pantalla.
- ⌨️ **Las teclas que ya conoces**: presets que conservan las teclas de Norton Commander / Far, el Explorador de Windows / VS Code / Total Commander, Neovim y yazi, una lista de atajos con búsqueda donde cualquier tecla se puede cambiar, y los comandos de los plugins en el mismo mapa de teclas ([help/es/keyboard_shortcuts.md](help/es/keyboard_shortcuts.md)).
- 🔌 **Sistema de Complementos Extensible**: Plugins Lua con sandbox (`mlua`), diálogos reales confirm/input/which, `pairee.cx` + userdata `File`, extras de `pairee.fs` y `pairee.Command` con streaming. API Lua **v1.3.0** ([docs/api/lua](docs/api/lua/README.md)).
- 🧰 **Herramientas Avanzadas**: Búsqueda integrada por nombre/contenido, comparación de carpetas, menú de comandos personalizados del usuario, gestor de procesos del sistema y visor de atributos.
- 📦 **Actualizaciones Automáticas Inteligentes**: Comprobaciones seguras de actualización con detección automática entre 13 métodos de instalación, con sumas SHA-256 y verificación obligatoria de la firma minisign.
- 🌐 **Traducciones Centralizadas**: Motor de traducción centralizado que soporta inglés y español, diseñado para extenderse fácilmente a nuevos idiomas.

---

## Estado del Proyecto

Beta pública. Lo suficientemente estable para su uso diario. Pairee está bajo desarrollo activo, y cualquier sugerencia o contribución es bienvenida.

---

## 📂 Estructura del Proyecto

```text
Pairee/
├── Cargo.toml                     # Configuración de Cargo
├── install.sh / install.ps1       # Instaladores rápidos
├── docs/
│   ├── IMPROVEMENT_PLAN.md        # Plan de mejora y progreso
│   ├── README.md                  # Índice de documentación (estado)
│   ├── api/lua/                   # API Lua versionada (semver)
│   └── technical/                 # Arquitectura, transfer, plugins, packaging
├── tests/
│   └── plugin_acceptance/         # Plugins Lua que ejecuta CI (`cargo test`)
├── help/
│   ├── en/                        # Ayuda in-app (inglés)
│   └── es/                        # Ayuda in-app (español)
├── lang/                          # Traducciones UI
├── keymaps/                       # Presets de teclado
├── .agents/                       # Directrices y skills para agentes
└── src/
    ├── main.rs                    # Entrada
    ├── app/                       # Bucle, acciones, estado
    ├── config/                    # Settings, temas, i18n
    ├── fs/                        # FS + transfer engine
    ├── git/                       # Integración Git
    ├── keybindings/               # Resolver y presets
    ├── plugin/                    # Runtime Lua y registro
    ├── ui/                        # Render Ratatui
    ├── terminal/                  # Backend y launcher
    └── update/                    # Auto-actualización
```

---

## 🛠️ Inicio Rápido

### Instalación (Instaladores Rápidos)

Pairee ofrece binarios precompilados de forma automática mediante GitHub Actions (Windows x64 y arm64, Linux x64 y arm64 con builds MUSL estáticos más un build GNU x64, macOS x64 y arm64). Puedes instalarlos al instante desde tu terminal:

* **Linux (enlace estático, ejecutable independiente, x86_64 y aarch64):**
  * **Lanzamiento Estándar:**
    ```bash
    curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh
    ```
    El instalador verifica la firma de la versión con [`minisign`](https://jedisct1.github.io/minisign/): el instalado o, si no está, la compilación oficial, que descarga solo (versión y SHA-256 fijados en el script). Agregá `--insecure-skip-signature` (`... | sh -s -- --insecure-skip-signature`) para aceptar la descarga solo con su checksum SHA-256.

    El programa va a `~/.local/lib/FittyAr/Pairee` con el comando `pairee` en `~/.local/bin`, que el instalador agrega al `PATH` de bash, zsh, fish, ksh y tcsh (y además en `/usr/local/bin` si se ejecuta como root), así `pairee` arranca desde cualquier terminal nueva sin pasos extra (una instalación de una versión anterior, con el programa en `~/.local/bin`, la mueve allí el instalador o la próxima autoactualización); las traducciones y la ayuda van a la carpeta de configuración.

    `sudo pairee` no encuentra `~/.local/bin` (el `secure_path` de sudo lo deja afuera), así que el instalador ofrece una copia que pertenece a root en `/usr/local/lib/FittyAr/Pairee`, enlazada desde `/usr/local/bin/pairee` (con `--sudo-command` la crea sin preguntar); las instalaciones siguientes la actualizan y `uninstall` la quita. Ejecutado como root, Pairee guarda su configuración en el home de root, nunca en el tuyo.
  * **Compilar desde Código Fuente (Modo Debug):**
    ```bash
    curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh -s -- debug
    ```
  * **Desinstalar:**
    ```bash
    curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh -s -- uninstall
    ```

* **Windows (PowerShell):**
  * **Lanzamiento Estándar:**
    ```powershell
    irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1 | iex
    ```
    El instalador verifica la firma de la versión con la compilación oficial de [`minisign`](https://jedisct1.github.io/minisign/), que descarga por su cuenta (versión y SHA-256 fijados en el script). Para aceptar la descarga solo con su checksum SHA-256, pasá `-InsecureSkipSignature`:
    ```powershell
    & ([scriptblock]::Create((irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1))) -InsecureSkipSignature
    ```

    El programa va a `%LOCALAPPDATA%\Programs\FittyAr\Pairee` (agregado al `PATH` del usuario) y la configuración, las traducciones y la ayuda a `%APPDATA%\FittyAr\Pairee`.
  * **Compilar desde Código Fuente (Modo Debug):**
    ```powershell
    & ([scriptblock]::Create((irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1))) -DebugMode
    ```
  * **Desinstalar:**
    ```powershell
    & ([scriptblock]::Create((irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1))) -Uninstall
    ```

### Compilar desde el Código Fuente

#### Prerrequisitos
Asegúrate de tener instalado [Rust](https://www.rust-lang.org/tools/install) **1.96 o superior** (edition 2024 / MSRV en `Cargo.toml`).

#### Compilar y Ejecutar
```bash
# Clonar el repositorio
git clone https://github.com/FittyAr/Pairee.git
cd Pairee

# Ejecutar Pairee en modo de desarrollo
cargo run

# Compilar el binario optimizado para producción
cargo build --release
```

### Línea de Comandos e Integración con la Shell
```sh
pairee [--cwd-file <archivo>] [--print-cwd] [--no-mouse] [--standalone] [<carpeta izquierda> [<carpeta derecha>]]
```
La última sesión (pestañas, carpetas, diseño) se restaura al iniciar (ajuste **Restaurar la última sesión**); las carpetas dadas en la línea de comandos tienen prioridad. `--cwd-file` escribe la carpeta del panel activo al salir, para que una función de la shell pueda hacer `cd` allí:

```sh
# bash / zsh (~/.bashrc, ~/.zshrc)
p() {
  local tmp dir
  tmp="$(mktemp)"
  command pairee --cwd-file "$tmp" "$@"
  dir="$(cat -- "$tmp")"
  [ -n "$dir" ] && [ "$dir" != "$PWD" ] && cd -- "$dir"
  rm -f -- "$tmp"
}
```

```fish
# fish (~/.config/fish/functions/p.fish)
function p
    set tmp (mktemp)
    command pairee --cwd-file $tmp $argv
    set dir (cat -- $tmp)
    if test -n "$dir"; and test "$dir" != "$PWD"
        cd -- $dir
    end
    rm -f -- $tmp
end
```

```powershell
# PowerShell ($PROFILE)
function p {
    $tmp = New-TemporaryFile
    pairee --cwd-file $tmp.FullName @args
    $dir = Get-Content -Raw -LiteralPath $tmp.FullName
    if ($dir -and $dir -ne $PWD.Path) { Set-Location -LiteralPath $dir }
    Remove-Item -LiteralPath $tmp.FullName
}
```

### Iniciar en Modo Independiente
Ejecutá `pairee --standalone` (desde el código fuente: `cargo xtask dev standalone`) para arrancar la aplicación en una ventana de consola dedicada. `cargo xtask dev` lista las demás acciones de desarrollo.

---

## ⚙️ Rutas de Configuración y Datos

Pairee almacena configuraciones, temas y registros de depuración en los directorios estándares del sistema:

* **Windows:** `%APPDATA%/FittyAr/Pairee/config` y `%APPDATA%/FittyAr/Pairee/cache`
* **Linux:** `~/.config/FittyAr/Pairee`, `~/.cache/FittyAr/Pairee` y `~/.local/share/FittyAr/Pairee` (datos de los complementos), respetando `XDG_CONFIG_HOME`, `XDG_CACHE_HOME` y `XDG_DATA_HOME`
* **macOS:** `~/Library/Application Support/FittyAr/Pairee` y `~/Library/Caches/FittyAr/Pairee`

Todas las apps de FittyAr comparten la carpeta `FittyAr`. Las carpetas de versiones anteriores (`%APPDATA%/pairee`, `~/.config/pairee`, `~/.cache/pairee`, `~/.local/share/pairee`, `com.pairee.Pairee` en macOS) se mueven a su lugar en el primer inicio; no se borra nada, y una carpeta que no se puede mover se sigue usando.

Los registros se añaden a `app.log` dentro de la carpeta cache, permitiéndote resolver problemas sin ensuciar la interfaz TUI. Al superar 5 MiB el archivo pasa a `app.log.1`. Las versiones release registran a nivel `info`; ejecutá con `RUST_LOG=debug` para más detalle.

---

## 📖 Más Información

Para manuales detallados, principios de diseño y opciones de configuración, consulta los siguientes documentos:

| Tema | Inglés | Español |
| :--- | :--- | :--- |
| **Wiki del Proyecto** | [DeepWiki](https://deepwiki.com/FittyAr/Pairee) | [DeepWiki](https://deepwiki.com/FittyAr/Pairee) |
| **Índice de docs y estado** | [docs/README.md](docs/README.md) | [docs/README.md](docs/README.md) |
| **Plan de mejora** | [IMPROVEMENT_PLAN.md](docs/IMPROVEMENT_PLAN.md) | [IMPROVEMENT_PLAN.md](docs/IMPROVEMENT_PLAN.md) |
| **Referencia de Funciones** | [Features Manual](help/en/features.md) | [Manual de Funciones](help/es/features.md) |
| **Arquitectura del Sistema** | [Architecture Guide](docs/technical/architecture_en.md) | [Guía de Arquitectura](docs/technical/architecture_es.md) |
| **Configuración y Opciones** | [User Guide](help/en/user_guide.md) | [Guía de Usuario](help/es/user_guide.md) |

---

## 📄 Licencia

Este proyecto está bajo la Licencia Pública General de GNU v3 (GPL v3). Consulta el archivo [LICENSE](LICENSE) para más detalles.
