# NCRust

[![Licencia: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
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

---

## 📂 Estructura del Proyecto

```text
NCRust/
├── Cargo.toml                     # Configuración de Cargo
├── agents.md                      # Directrices para desarrolladores de IA
├── LICENSE                        # Licencia MIT
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

### Prerrequisitos
Asegúrate de tener instalado [Rust](https://www.rust-lang.org/tools/install).

### Compilar y Ejecutar
```bash
# Clonar el repositorio
git clone https://github.com/FittyAr/NCRust.git
cd NCRust

# Ejecutar NCRust en modo de desarrollo
cargo run

# Compilar el binario optimizado para producción
cargo build --release
```

### Iniciar en Modo Independiente
Puedes revisar los scripts de lanzamiento `run.bat` (Windows) o `run.sh` (Linux/macOS) para arrancar la aplicación en una ventana de consola dedicada.

---

## ⚙️ Rutas de Configuración y Datos

NCRust almacena configuraciones, temas y registros de depuración en los directorios estándares del sistema:

* **Windows:** `%APPDATA%/ncrust/config` y `%APPDATA%/ncrust/cache`
* **Linux/macOS:** `~/.config/ncrust` y `~/.cache/ncrust`

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

Este proyecto está bajo la Licencia MIT. Consulta el archivo [LICENSE](LICENSE) para más detalles.
