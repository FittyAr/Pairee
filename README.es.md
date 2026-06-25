# Pairee

> Tu mundo, en dos paneles.

[![Licencia: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

Un gestor de archivos para terminal moderno, multiplataforma y altamente modular, inspirado en los clásicos **Norton Commander** y **Far Manager**. Desarrollado en Rust utilizando `ratatui` y `crossterm`.

---

## 🚀 Características Clave

* **Interfaz de Doble Panel:** El clásico diseño de doble panel para navegar eficientemente por directorios y realizar operaciones en lote.
* **Inicio en Modo Dual:** Ejecútalo como una aplicación de consola estándar o lanza una ventana de terminal optimizada e independiente.
* **Operaciones Asíncronas en Segundo Plano:** Tareas pesadas (Copiar, Mover, Borrado Seguro, Eliminar) se procesan en un grupo de hilos independiente (`tokio`), manteniendo la UI responsiva con barras de progreso en tiempo real.
* **Resolutor de Atajos de Teclado Flexible:** Perfiles integrados para controles de Norton Clásico, Vim y navegación moderna.
* **Alta Extensibilidad:** Estructura modular alineada con el Principio de Responsabilidad Única, cargadores de temas visuales y soporte para complementos.
* **Localización y Temas:** Motor centralizado para traducciones que soporta inglés, español y temas personalizados (Slate, Blue, etc.).
* **Funcionalidades Avanzadas:** Búsqueda de archivos por nombre/contenido, comparación de carpetas, menú personalizado de comandos de usuario, administrador de procesos del sistema y visor de atributos.
* **Actualización Automática Inteligente:** Sistema integrado de actualizaciones seguras que detecta de forma automática la vía de instalación del usuario de entre 13 métodos (como gestores de paquetes o descargas directas) y descarga/instala validando firmas SHA-256.

---

## 📂 Estructura del Proyecto

```text
Pairee/
├── Cargo.toml                     # Configuración de Cargo
├── agents.md                      # Directrices para desarrolladores de IA
├── install.sh                     # Script de instalación para Linux (compatible con curl)
├── install.ps1                    # Script de instalación para Windows (compatible con PowerShell)
├── LICENSE                        # Licencia GNU GPL v3
├── README.md                      # Índice de documentación en inglés
├── README.es.md                   # Índice de documentación en español (Este archivo)
├── docs/                          # Documentación para desarrolladores
│   └── technical/
│       ├── architecture_en.md     # Arquitectura y diseño del código (Inglés)
│       └── architecture_es.md     # Arquitectura y diseño del código (Español)
├── help/                          # Documentación de ayuda al usuario (cargada por F1)
│   ├── features_en.md             # Manual de características (Inglés)
│   ├── features_es.md             # Manual de características (Español)
│   ├── user_guide_en.md           # Guía de configuración y personalización (Inglés)
│   └── user_guide_es.md           # Guía de configuración y personalización (Español)
└── src/                           # Código fuente
    ├── main.rs                    # Punto de entrada de la aplicación
    ├── app/                       # Bucles de eventos, acciones y gestión de estado
    ├── config/                    # Configuración TOML, temas y traducciones
    ├── fs/                        # Operaciones del sistema de archivos y canales asíncronos
    ├── keybindings/               # Mapeo de entradas y motor de atajos de teclado
    ├── ui/                        # Paneles, menús y ventanas emergentes de Ratatui
    └── terminal/                  # Controlador de pantalla raw y configuración del backend
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
Asegúrate de tener instalado [Rust](https://www.rust-lang.org/tools/install).

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
| **Referencia de Funciones** | [Features Manual](help/features_en.md) | [Manual de Funciones](help/features_es.md) |
| **Arquitectura del Sistema** | [Architecture Guide](docs/technical/architecture_en.md) | [Guía de Arquitectura](docs/technical/architecture_es.md) |
| **Configuración y Opciones** | [User Guide](help/user_guide_en.md) | [Guía de Usuario](help/user_guide_es.md) |

---

## 📄 Licencia

Este proyecto está bajo la Licencia Pública General de GNU v3 (GPL v3). Consulta el archivo [LICENSE](LICENSE) para más detalles.
