# Pairee

> Tu mundo, en dos paneles.

[![Licencia: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/FittyAr/Pairee)
![CodeRabbit Pull Request Reviews](https://img.shields.io/coderabbit/prs/github/FittyAr/Pairee?utm_source=oss&utm_medium=github&utm_campaign=FittyAr%2FPairee&labelColor=171717&color=FF570A&link=https%3A%2F%2Fcoderabbit.ai&label=CodeRabbit+Reviews)

## Pairee - ⚡ Gestor de Archivos de Terminal Moderno de Doble Panel

Pairee es un gestor de archivos para terminal moderno, multiplataforma y altamente modular, inspirado en los clásicos **Norton Commander** y **Far Manager**. Desarrollado en Rust utilizando `ratatui` y `crossterm`, su objetivo es proporcionar una experiencia de gestión de archivos eficiente, rápida y extensible directamente en tu consola.

- 🚀 **Soporte Asíncrono Completo**: Todas las operaciones pesadas del sistema de archivos (Copiar, Mover, Borrado Seguro, Eliminar) se procesan en segundo plano (`tokio`), manteniendo la interfaz perfectamente fluida y responsiva.
- 💪 **Planificación de Tareas Asíncronas**: Seguimiento del progreso en tiempo real con ventanas emergentes de barras de progreso y cancelación de tareas para trabajadores concurrentes.
- 🎨 **Temas Visuales y Diseños**: Cargador de temas visuales personalizados (como Slate, Blue, entre otros) y distribución adaptable del diseño de pantalla.
- ⚙️ **Atajos de Teclado Flexibles**: Resolutor de atajos dinámico con perfiles predefinidos para Norton Clásico (F1-F10), navegación Vim y teclas modernas.
- 🔌 **Sistema de Complementos Extensible**: Plugins Lua con sandbox (`mlua`), diálogos reales confirm/input/which, `pairee.cx` + userdata `File`, extras de `pairee.fs` y `pairee.Command` con streaming. API Lua **v1.0** ([docs/api/lua](docs/api/lua/README.md)).
- 🧰 **Herramientas Avanzadas**: Búsqueda integrada por nombre/contenido, comparación de carpetas, menú de comandos personalizados del usuario, gestor de procesos del sistema y visor de atributos.
- 📦 **Actualizaciones Automáticas Inteligentes**: Comprobaciones seguras de actualización con detección automática entre 13 métodos de instalación y validación de firmas SHA-256.
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

NCRust ofrece binarios precompilados de forma automática mediante GitHub Actions (compatibles con Windows MSVC, Linux GNU y Linux MUSL estático). Puedes instalarlos al instante desde tu terminal:

* **Linux (enlace estático, ejecutable independiente):**
  * **Lanzamiento Estándar:**
    ```bash
    curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh
    ```
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
  * **Compilar desde Código Fuente (Modo Debug):**
    ```powershell
    irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1 | iex -Arguments debug
    ```
  * **Desinstalar:**
    ```powershell
    irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1 | iex -Arguments uninstall
    ```

### Compilar desde el Código Fuente

#### Prerrequisitos
Asegúrate de tener instalado [Rust](https://www.rust-lang.org/tools/install) **1.85 o superior** (edition 2024 / MSRV en `Cargo.toml`).

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

### Iniciar en Modo Independiente
Puedes revisar los scripts de lanzamiento `run.bat` (Windows) o `run.sh` (Linux/macOS) para arrancar la aplicación en una ventana de consola dedicada.

---

## ⚙️ Rutas de Configuración y Datos

Pairee almacena configuraciones, temas y registros de depuración en los directorios estándares del sistema:

* **Windows:** `%APPDATA%/pairee/config` y `%APPDATA%/pairee/cache`
* **Linux/macOS:** `~/.config/pairee` y `~/.cache/pairee`

Los registros de depuración se guardan en `app.log` dentro de la carpeta cache, permitiéndote resolver problemas sin ensuciar la interfaz TUI.

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
