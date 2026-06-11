# Guía de Usuario y Manual de Configuración de Pairee

Esta guía cubre la instalación, ajustes de personalización, temas visuales, atajos de teclado y configuraciones de archivos para **Pairee**.

---

## 🛠️ 1. Compilación e Instalación

### Compilación
Genera el binario ejecutable desde la ruta raíz del proyecto utilizando Cargo:
```bash
# Compilar en modo de desarrollo (incluye símbolos)
cargo build

# Compilar binario optimizado para producción
cargo build --release
```
El archivo compilado se genera en:
* **Windows:** `target/release/pairee.exe`
* **Linux/macOS:** `target/release/pairee`

---

## ⌨️ 2. Atajos de Teclado del Sistema

### 2.1 Navegación General
| Tecla / Atajo | Acción |
| :--- | :--- |
| `Tab` | Cambia el foco de cursor entre los paneles izquierdo y derecho. |
| `Flechas Arriba / Abajo` | Desplaza la selección de archivos hacia arriba o hacia abajo. |
| `Re Pág / Av Pág` | Desplaza la lista un cuadro completo arriba o abajo. |
| `Inicio / Fin` | Ir al primer o último elemento de la lista. |
| `Ctrl+U` | Intercambia las rutas de los directorios entre el panel izquierdo y derecho. |
| `Ctrl+H` | Muestra u oculta archivos y carpetas ocultas. |
| `Ctrl+R` | Vuelve a cargar y refresca el contenido de la carpeta activa. |
| `Ctrl+\` | Abre la lista de Favoritos de directorios (Hotlist). |
| `Alt+F8` | Abre el historial de comandos de la consola. |
| `Alt+F12` | Abre el historial de navegación de carpetas. |
| `Ctrl+PgUp` / `Ctrl+PgDn` | Cambia o selecciona la unidad de disco activa. |
| `Alt+F1` / `Alt+F2` | Abre la selección de unidad para el panel Izquierdo / Derecho. |

### 2.2 Gestión de Pantallas (Screens y Pestañas)
| Tecla / Atajo | Acción |
| :--- | :--- |
| `Ctrl+Tab` / `Ctrl+Derecha` | Salta al foco de la siguiente pantalla (Screen) activa. |
| `Ctrl+Shift+Tab` / `Ctrl+Izquierda` | Regresa al foco de la pantalla anterior. |
| `F2 -> Comandos -> Lista de pantallas` | Muestra el menú flotante con el carrusel de pestañas abiertas. |

### 2.3 Visibilidad de Paneles
| Tecla / Atajo | Acción |
| :--- | :--- |
| `Ctrl+F1` | Muestra u oculta el Panel Izquierdo. |
| `Ctrl+F2` | Muestra u oculta el Panel Derecho. |
| `Ctrl+O` | Oculta / Muestra ambos paneles. |

### 2.4 Acciones sobre Archivos
| Tecla | Acción |
| :--- | :--- |
| `F1` | Abre el menú de ayuda y atajos de teclado. |
| `F2` | Abre el menú de acciones de la barra superior. |
| `F3` | Abre el visor interno de archivos (modos texto/hexadecimal). |
| `F4` | Abre el editor interno de archivos. |
| `F5` | Copia los archivos seleccionados o marcados hacia la ruta del panel pasivo. |
| `F6` | Mueve o renombra los archivos seleccionados hacia la ruta del panel pasivo. |
| `F7` | Crea una nueva carpeta (MkDir). |
| `F8` | Elimina los archivos seleccionados o marcados. |
| `F9` | Activa la barra de menú superior. |
| `F10` | Cierra la aplicación. |
| `Esc` | Cierra diálogos de confirmación o limpia la consola inferior. |
| `Shift+F10` | Abre el menú de acciones contextuales. |
| `Ctrl+L` / `Alt+F6` | Abre el diálogo para crear Enlaces Duros (Hardlink) o Simbólicos (Symlink). |
| `Ctrl+D` | Añade o modifica la descripción del archivo (`Descript.ion`). |

### 2.5 Selección Múltiple
| Tecla | Acción |
| :--- | :--- |
| `Insert` / `Espacio` | Marca o desmarca un archivo individual para operaciones en lote. |
| `+` (Teclado numérico) | Selecciona todos los archivos que coincidan con un patrón de máscara (ej. `*.txt`). |
| `-` (Teclado numérico) | Deselecciona archivos que coincidan con el patrón. |
| `*` (Teclado numérico) | Invierte el estado de selección de la lista completa del panel activo. |

---

## ⚙️ 3. Opciones del Diálogo de Configuración (`F2 -> Opciones -> Configuración`)

Las opciones de configuración se agrupan en las siguientes pestañas:

### Pestaña 0: Ajustes del Sistema
* **Delete to Recycle Bin:** Envía los archivos eliminados a la papelera del sistema en lugar de borrarlos permanentemente.
* **Use system copy routine:** Delega las operaciones de copia a las rutinas del SO nativo en lugar del motor asíncrono de Pairee.
* **Copy files opened for writing:** Habilita la copia de archivos abiertos por otras aplicaciones.
* **Sorting collation:** Algoritmo de ordenación. Admite `linguistic` (orden alfabético natural) o `binary`.
* **Treat digits as numbers:** Permite que `archivo2` aparezca ordenado antes que `archivo10`.
* **Case sensitive sort:** Activa el ordenamiento sensible a mayúsculas y minúsculas.
* **Scan symbolic links:** Sigue la ruta de los enlaces simbólicos al listar.
* **Save commands history:** Guarda el registro de comandos ejecutados en consola.
* **Save folders history:** Guarda el historial de carpetas visitadas.
* **Save view and edit history:** Recuerda los archivos abiertos recientemente en el editor o visor.
* **Auto save setup:** Guarda la configuración actual automáticamente al salir.

### Pestaña 1: Ajustes de Paneles
* **Show hidden and system files:** Toggles archivos ocultos y de sistema.
* **Highlight files:** Colorea los archivos según su tipo de extensión.
* **Select folders:** Permite seleccionar directorios con máscaras comodín.
* **Sort folder names by extension:** Aplica criterios de ordenación a las extensiones de carpetas.
* **Show column titles:** Muestra la cabecera de las columnas en los paneles.
* **Show status line:** Muestra el contador de archivos marcados.
* **Show scrollbar:** Muestra barras de desplazamiento vertical.
* **Show ".." in root folders:** Muestra el enlace al directorio padre en la raíz del disco.

### Pestaña 2: Ajustes de Interfaz
* **Clock:** Muestra el reloj digital en la esquina superior derecha.
* **Show key bar:** Muestra la fila inferior de atajos F1-F10.
* **Always show the menu bar:** Mantiene el menú superior visible todo el tiempo.
* **Show total copy progress indicator:** Muestra la barra de progreso para tareas de copiado.
* **Show total delete progress indicator:** Muestra la barra de progreso para tareas de eliminación.
* **Keybindings preset:** Cambia el perfil de teclado: `"norton"`, `"vim"` o `"modern"`.

### Pestaña 4: Ajustes de Idioma y Plugins
* **Main language:** Ajusta el archivo de traducción activa (ej. `"English"` o `"Spanish"`).
* **OEM plugins support:** Habilita la carga de complementos OEM.

### Pestaña 5: Ajustes del Editor y Visor
* **Use external editor for F4:** Delega la edición a un programa externo.
* **Editor command:** Comando del editor externo (ej. `nano %f`).
* **Use external viewer for F3:** Delega el visor a un programa externo.
* **Viewer command:** Comando del visor externo (ej. `less %f`).
* **Tab size:** Tamaño en espacios del tabulador.
* **Show line numbers:** Muestra números de línea en el editor.

### Pestaña 6: Ajustes de Colores
* **Theme:** Elige el tema gráfico de la interfaz (Slate, Blue, High Contrast).

---

## 🎨 4. Creación de Temas TOML

Los temas se cargan de `%APPDATA%/pairee/config/themes/` (Windows) o `~/.config/pairee/themes/` (Linux/macOS).

### Mapa de Propiedades
```toml
[panel]
border = "Blue"              # Color del borde del panel
background = "Black"          # Fondo interno del panel
file_selected = "Yellow"      # Color de archivos marcados
file_directory = "Cyan"       # Color de carpetas
file_executable = "Green"     # Color of binaries/scripts

[menu]
background = "Blue"          # Fondo del menú superior
selected = "White"            # Texto seleccionado
```
Colores admitidos: `Black`, `Red`, `Green`, `Yellow`, `Blue`, `Magenta`, `Cyan`, `White`, `Gray`, `DarkGray`, `Reset` o colores hexadecimales (`#RRGGBB`).

---

## 🌐 5. Uso de Pairee a través de SSH y Teclas Modificadoras (Ctrl / Alt)

Al utilizar **Pairee** de forma remota a través de una conexión SSH, notarás que mantener presionado `Ctrl` o `Alt` no actualiza automáticamente la barra inferior de teclas F. Esta es una limitación física e histórica de las terminales tradicionales y del protocolo SSH, los cuales solo transmiten flujos de texto cuando se presiona una combinación completa (no envían ninguna señal cuando las teclas modificadoras se presionan o liberan por sí solas).

Para resolver esta limitación, he diseñado varias alternativas para que puedas usar y visualizar estos atajos sin problemas:

### 1. Rotación Manual de Modificadores (Sin Aplicaciones de Terceros)
Dentro de **Pairee**, puedes presionar **`Ctrl+p`** (o `Ctrl+P`) para rotar visualmente los estados de la barra de teclas F:
* **Primera pulsación**: Bloquea la barra en la vista de **CONTROL** (ej. F3: Nombre, F4: Extensión).
* **Segunda pulsación**: Bloquea la barra en la vista de **ALT** (ej. F3: Ver, F4: Editar).
* **Tercera pulsación**: Restaura la vista por defecto de la barra.

*Nota: Todas las combinaciones de teclas siguen estando 100% funcionales aunque la barra no las muestre en ese momento. Por ejemplo, al presionar `Ctrl+F3` se ordenará la lista por nombre y al presionar `Alt+F1` se abrirá el selector de discos izquierdo al instante.*

### 2. Detección Física en Tiempo Real (Mediante Reenvío X11)
Si deseas que la barra cambie automáticamente al mantener presionados físicamente los botones `Ctrl` o `Alt`, puedes activar el **Reenvío X11** (X11 Forwarding) en tu conexión SSH. Al hacer esto, **Pairee** consultará al servidor gráfico X11 local el estado físico de tu teclado.

A continuación, detallo las aplicaciones de terceros recomendadas y sus configuraciones según tu sistema operativo:

#### 💻 Servidores en Windows
* **MobaXterm (Recomendado y más sencillo)**:
  Incluye un servidor X11 integrado de fábrica. Solo debes crear una nueva sesión SSH y MobaXterm habilitará el reenvío gráfico automáticamente.
* **Windows Terminal / PowerShell / CMD (con VcXsrv)**:
  1. Instala **VcXsrv** (o **Xming**).
  2. Ejecuta **XLaunch** (VcXsrv) con la siguiente configuración:
     * *Multiple windows*
     * Display number: `0`
     * **Crucial**: Marca la opción **"Disable access control"** (para dar permisos de conexión al contenedor o servidor remoto).
  3. Conéctate desde tu consola ejecutando:
     ```cmd
     ssh -Y usuario@servidor -p puerto
     ```
* **PuTTY**:
  1. Despliega **Connection** -> **SSH** -> **X11** en el panel de configuración.
  2. Marca la casilla **"Enable X11 forwarding"**.
  3. Escribe `localhost:0` en **X display location**.
  4. Asegúrate de tener VcXsrv ejecutándose de fondo antes de abrir la conexión.

#### 🍎 Servidores en macOS
* **XQuartz**:
  1. Instala **XQuartz**.
  2. Ejecuta XQuartz, abre *Preferencias* -> *Seguridad* y marca la casilla **"Allow connections from clients"**.
  3. Conéctate desde la terminal usando:
     ```bash
     ssh -Y usuario@servidor -p puerto
     ```

#### 🐧 Servidores en Linux
* Los sistemas Linux tienen soporte X11 nativo de fábrica. Solo debes conectarte con:
  ```bash
  ssh -Y usuario@servidor -p puerto
  ```
