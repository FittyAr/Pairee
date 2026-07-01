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

## 🔌 4. Gestor de Plugins (Manual de Usuario)

Pairee cuenta con un potente motor de plugins integrado basado en el lenguaje de scripting Lua. Los plugins se ejecutan dentro de un entorno seguro y aislado (sandbox) para proteger la integridad de tu sistema de archivos.

### 4.1 Abrir el Gestor de Plugins
Puedes acceder al gestor de plugins presionando la tecla **`F11`** (o seleccionando `F9 -> Opciones -> Administrador de Plugins`). 

La ventana emergente presenta dos pestañas principales entre las que puedes cambiar presionando la tecla **`Tab`**:

---

#### Pestaña 1: Plugins Instalados
Muestra una lista con todos los plugins que se han cargado y están instalados localmente.

##### Indicadores de Estado (Badges):
* **`[P]` (Pinned):** El plugin está anclado a su versión actual. No se actualizará de manera automática al ejecutar actualizaciones globales.
* **`[T]` (Trusted):** El plugin tiene permisos de ejecución extendidos ("Modo de Confianza"). Los plugins que no muestran esta etiqueta se ejecutan en modo estrictamente restringido (sandbox), sin acceso a la red ni a comandos arbitrarios del sistema operativo.
* **`[▲]` (Update Available):** Indica que existe una versión más reciente de este plugin disponible en el repositorio central.

##### Controles de Teclado:
* **`t` / `T`:** Cambia (activa/desactiva) el estado de confianza (Trusted) del plugin seleccionado en la configuración.
* **`p` / `P`:** Ancla (Pin) o desancla la versión actual del plugin en `plugins.lock`.
* **`u`:** Actualiza el plugin seleccionado a la versión más reciente en segundo plano. Recibirás una notificación en la parte superior cuando finalice.
* **`U`:** Actualiza todos los plugins instalados (que no estén anclados) de forma masiva en segundo plano.
* **`Del` / `d` / `D`:** Desinstala por completo el plugin seleccionado y lo elimina de tu disco.

---

#### Pestaña 2: Buscar en el Registro (Search Registry)
Permite buscar e instalar nuevos complementos desde el repositorio central oficial de Pairee.

##### Controles de Teclado:
* **`/`:** Activa la edición del cuadro de búsqueda en la parte superior (el borde se iluminará en amarillo indicando que está listo para escribir).
* **`Escribir caracteres` / `Backspace`:** Modifica el texto de tu consulta de búsqueda.
* **`Enter`:** Envía la consulta al repositorio remoto para actualizar la lista de resultados coincidentes. Al finalizar, la edición se cierra y vuelves a la navegación del listado.
* **`i` / `I`:** Instala el plugin seleccionado del listado de resultados de búsqueda. El proceso se ejecuta en segundo plano y te mostrará un aviso tipo toast cuando se complete correctamente.

---

### 4.2 Desarrolladores de Plugins
Si deseas escribir tus propios complementos, auditar la arquitectura de seguridad o colaborar con el registro oficial, te invitamos a habilitar el **Modo Desarrollador** en el panel de Configuración (F9 -> Pestaña 4: Idioma y Plugins). Esto activará la tercera pestaña (**Herramientas de Desarrollo / Developer Tools**) en el Gestor de Plugins (F11) y te permitirá configurar tu propio directorio de desarrollo personalizado.

#### Herramientas de Desarrollo Disponibles (F11 - Pestaña 3):
1. **Inicializar plantilla:** Crea un esqueleto de plugin en tu directorio de desarrollo con los archivos mínimos necesarios (`manifest.toml`, `main.lua` y traducciones en `lang/en.toml`).
2. **Verificar (Lint):** Escanea tu carpeta de desarrollo y realiza auditorías de seguridad e integridad sobre los archivos Lua y de manifiesto. Detecta llamadas a funciones potencialmente peligrosas (como ejecución de subprocesos externos u operaciones de red no declaradas).
3. **Empaquetar:** Escanea tu carpeta de desarrollo y genera automáticamente los bloques de metadatos con los hashes SHA-256 de los archivos para incluirlos en el índice del registro.
4. **Instalar plugin local en desarrollo:** Copia de inmediato tus plugins en desarrollo a la carpeta de ejecución local de Pairee y los registra en el archivo `plugins.lock`. Esto hace que aparezcan al instante en tu lista de "Plugins Instalados" (Pestaña 1) para que puedas probar sus funciones directamente en la interfaz.
5. **Enviar plugin (GitHub PR):** Ejecuta el asistente interactivo para subir tu plugin al repositorio central de Pairee.
   - **¿Para qué sirve el Token de Acceso Personal de GitHub (PAT)?**
     Para automatizar la subida, Pairee necesita interactuar con la API de GitHub en tu nombre. El Token de Acceso Personal sirve como credencial segura para autenticarte. Permite al programa realizar un fork del repositorio central oficial (`FittyAr/Pairee`), subir tu plugin empaquetado a una nueva rama y abrir una solicitud de extracción (Pull Request) de forma transparente.
   - **Nota sobre Seguridad:** El token nunca se almacena en el disco ni en variables de entorno persistentes; solo se utiliza de forma transitoria en memoria durante la transacción de red.

Para obtener guías técnicas en profundidad:
* 📚 [**Guía de Desarrollo de Plugins en GitHub**](https://github.com/FittyAr/Pairee/blob/master/docs/plugin-dev-guide.md)
* 🛠️ [**Diseño Técnico de la Arquitectura de Plugins**](https://github.com/FittyAr/Pairee/blob/master/docs/technical/plugin-system-design.md)
* 📂 [**Especificación del Registro Central de Distribución**](https://github.com/FittyAr/Pairee/blob/master/docs/technical/plugin-registry-spec.md)

---

## 📖 5. Manuales de Integración Avanzada

Para módulos más complejos y detallados, por favor consulta sus manuales específicos:
* **Conexión SSH y SFTP:** Consulta el [Manual de Conexiones SSH y SFTP](file:///home/fitty/GitHub/Pairee/help/ssh_sftp_es.md).
* **Integración con Git:** Consulta el [Manual de Integración con Git](file:///home/fitty/GitHub/Pairee/help/git_integration_es.md).
* **Detalle de Ajustes de Configuración:** Consulta el [Manual de Ajustes de Configuración](file:///home/fitty/GitHub/Pairee/help/configuration_details_es.md).
* **Atajos de Teclado del Sistema:** Consulta la [Guía de Atajos de Teclado](file:///home/fitty/GitHub/Pairee/help/keyboard_shortcuts_es.md).

