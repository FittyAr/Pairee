# Guía de Usuario y Configuración de Pairee

Esta guía cubre la compilación, instalación, personalizaciones y formatos de temas gráficos personalizados para **Pairee**.

---

## 🛠️ 1. Compilación e Instalación

### Compilación
Compila el ejecutable desde el directorio raíz usando Cargo:
```bash
# Compilación en modo Debug (incluye símbolos)
cargo build

# Compilación en modo Release (optimizado y sin logs de depuración)
cargo build --release
```
El archivo ejecutable compilado estará ubicado en:
* **Windows:** `target/release/pairee.exe`
* **Linux/macOS:** `target/release/pairee`

### Instalación
Puedes copiar el ejecutable a una ruta del sistema (ej. `/usr/local/bin/` o `C:\Windows\System32\`) o ejecutarlo directamente desde la carpeta compilada.
Asegúrate de colocar los directorios `lang/` y `help/` junto al ejecutable o en el directorio de recursos compartidos del sistema (`/usr/share/pairee/` en Linux) para que las traducciones y los manuales se carguen correctamente.

---

## 🎨 2. Temas de Color Personalizados (TOML)

Los archivos de tema se cargan desde `%APPDATA%/pairee/config/themes/` (Windows) o `~/.config/pairee/themes/` (Linux/macOS) en formato TOML.

### Estructura de Propiedades de Tema
```toml
[panel]
border = "Blue"              # Color del borde del panel
background = "Black"          # Fondo interno del panel
file_selected = "Yellow"      # Color de elementos marcados
file_directory = "Cyan"       # Color de carpetas
file_executable = "Green"     # Color de binarios y scripts

[menu]
background = "Blue"          # Fondo del menú superior
selected = "White"            # Color del texto seleccionado
```
Colores soportados: `Black`, `Red`, `Green`, `Yellow`, `Blue`, `Magenta`, `Cyan`, `White`, `Gray`, `DarkGray`, `Reset`, o valores hexadecimales personalizados (`#RRGGBB`).

---

## 🌐 3. Uso de Pairee sobre SSH y Teclas Modificadoras (Ctrl / Alt)

Al usar **Pairee** de forma remota a través de conexiones SSH, notarás que mantener presionados los botones físicos `Ctrl` o `Alt` no actualiza de forma automática la barra inferior de teclas F. Esta es una limitación inherente al protocolo SSH y a los terminales estándar, que solo envían secuencias de bytes cuando se realiza una combinación completa (no envían eventos cuando los modificadores se presionan o liberan por separado).

Para solventar esta limitación, se han implementado diferentes alternativas:

### 3.1 Rotación Manual de Modificadores (Sin Software de Terceros)
Dentro de **Pairee**, puedes presionar la combinación **`Ctrl+p`** (o `Ctrl+P`) para rotar visualmente los estados de la barra de teclas F inferior:
* **Primera pulsación**: Bloquea la barra en la vista de **CONTROL** (ej. F3: Nombre, F4: Extensión).
* **Segunda pulsación**: Bloquea la barra en la vista de **ALT** (ej. F3: Ver, F4: Editar).
* **Tercera pulsación**: Restaura la vista por defecto de la barra.

*Nota: Todas las combinaciones siguen funcionando perfectamente aunque la barra no las muestre. Por ejemplo, al presionar `Ctrl+F3` se ordenará la lista por nombre y al presionar `Alt+F1` se abrirá el selector de discos izquierdo al instante.*

### 3.2 Detección Física (Mediante Reenvío X11)
Si deseas que la barra cambie automáticamente al mantener presionados físicamente los botones `Ctrl` o `Alt`, puedes activar el **Reenvío X11** (X11 Forwarding) en tu conexión SSH. Al hacer esto, **Pairee** consultará al servidor gráfico X11 local el estado físico de tu teclado.

A continuación, se detalla cómo configurarlo según tu software de conexión:

#### 💻 Host en Windows
* **MobaXterm (Recomendado y más sencillo)**:
  Incluye un servidor X11 integrado de fábrica. Solo debes crear una nueva sesión SSH y MobaXterm habilitará el reenvío gráfico automáticamente.
* **Windows Terminal / PowerShell / CMD (con VcXsrv)**:
  1. Instala **VcXsrv** (o **Xming**).
  2. Ejecuta **XLaunch** (VcXsrv) con la siguiente configuración:
     - *Multiple windows*
     - Display number: `0`
     - **Crucial**: Marca la opción **"Disable access control"** (para dar permisos de conexión al contenedor o servidor remoto).
  3. Conéctate desde tu consola ejecutando:
     ```cmd
     ssh -Y usuario@servidor -p puerto
     ```
* **PuTTY**:
  1. Despliega **Connection** -> **SSH** -> **X11** en el panel de configuración.
  2. Marca la casilla **"Enable X11 forwarding"**.
  3. Escribe `localhost:0` en **X display location**.
  4. Asegúrate de tener VcXsrv ejecutándose de fondo antes de abrir la conexión.

#### 🍎 Host en macOS
* **XQuartz**:
  1. Instala **XQuartz**.
  2. Ejecuta XQuartz, abre *Preferencias* -> *Seguridad* y marca la casilla **"Allow connections from clients"**.
  3. Conéctate desde la terminal usando:
     ```bash
     ssh -Y usuario@servidor -p puerto
     ```

#### 🐧 Host en Linux
* Los sistemas Linux tienen soporte X11 nativo de fábrica. Solo debes conectarse con:
  ```bash
  ssh -Y usuario@servidor -p puerto
  ```

---

## 📖 4. Manuales de Integración Avanzada

Para módulos más complejos y detallados, por favor consulta sus manuales específicos:
* **Conexión SSH y SFTP:** Consulta el [Manual de Conexiones SSH y SFTP](file:///home/fitty/GitHub/Pairee/help/ssh_sftp_es.md).
* **Integración con Git:** Consulta el [Manual de Integración con Git](file:///home/fitty/GitHub/Pairee/help/git_integration_es.md).
* **Detalle de Ajustes de Configuración:** Consulta el [Manual de Ajustes de Configuración](file:///home/fitty/GitHub/Pairee/help/configuration_details_es.md).
* **Atajos de Teclado del Sistema:** Consulta la [Guía de Atajos de Teclado](file:///home/fitty/GitHub/Pairee/help/keyboard_shortcuts_es.md).
