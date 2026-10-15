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

En el **primer arranque** (config nueva) Pairee pregunta el preset de teclado: **Norton Commander**, **Standard** (teclas del Explorador de Windows / VS Code / Total Commander), **Neovim** o **yazi**. Enter elige; Esc deja Norton. Luego se cambia en Ajustes → Interfaz → preset de teclas.

---

## 🎨 2. Temas de Color Personalizados (TOML)

Los archivos de tema se cargan desde `%APPDATA%/pairee/config/themes/` (Windows) o `~/.config/pairee/themes/` (Linux/macOS) en formato TOML, un archivo por tema: `themes/<nombre>.toml` es el tema que se elige por nombre en los ajustes. Hay dos temas integrados, `slate` (el predeterminado) y `classic_blue`; un archivo con el mismo nombre reemplaza al integrado.

### Estructura de Propiedades de Tema
Los colores son claves planas al principio del archivo. `name` y todos los colores siguientes son **obligatorios**: un archivo al que le falta alguno (o con un error de sintaxis) se ignora, se usa el tema integrado y se informa del error. Solo `[syntax]` y `[ui]` son opcionales, y cualquier entrada que falte dentro de ellas conserva su valor por defecto.
```toml
name = "mi_tema"

panel_bg = "Reset"          # Fondo del panel
panel_fg = "White"          # Texto del panel
panel_border = "DarkGray"   # Borde del panel
selection_bg = "Blue"       # Fondo de la línea del cursor
selection_fg = "White"      # Texto de la línea del cursor
marked_fg = "Yellow"        # Elementos marcados
header_bg = "Reset"         # Fondo del encabezado de ruta
header_fg = "Cyan"          # Texto del encabezado de ruta
cli_bg = "Reset"            # Fondo de la línea de comandos
cli_fg = "White"            # Texto de la línea de comandos
fkey_num_fg = "White"       # Barra de teclas F: número
fkey_text_fg = "Black"      # Barra de teclas F: etiqueta
fkey_bg = "Cyan"            # Fondo de la barra de teclas F
popup_bg = "Black"          # Fondo de los diálogos
popup_fg = "White"          # Texto de los diálogos
popup_border = "DarkGray"   # Borde de los diálogos

[syntax]                    # Resaltado del editor y del visor (opcional)
comment = "DarkGray"
string = "Green"
keyword = "LightMagenta"
constant = "LightRed"       # Números, true / false, escapes
function = "LightYellow"
type_name = "LightCyan"
operator = "Gray"
tag = "LightBlue"           # Etiquetas de marcado, títulos, enlaces

[ui]                        # Colores semánticos de diálogos, etiquetas y barras de progreso (opcional)
accent = "Cyan"
warning = "Yellow"
danger = "Red"
error = "LightRed"
success = "Green"
muted = "DarkGray"
# también: accent_alt, warning_alt, success_alt, secondary, info, link, special, strong, badge_fg, alert_bg
```
Los valores mostrados son los del tema integrado `slate`. El significado de cada entrada de `[ui]` está en *Grupos de color* en la referencia de configuración, donde esos mismos colores también se editan desde Pairee.

Colores soportados: `Black`, `Red`, `Green`, `Yellow`, `Blue`, `Magenta`, `Cyan`, `Gray`, `DarkGray`, `White`, las variantes `Light` (`LightRed`, `LightGreen`, `LightYellow`, `LightBlue`, `LightMagenta`, `LightCyan`), `Reset` (el color propio del terminal) y hexadecimal `#RRGGBB`. Los nombres no distinguen mayúsculas; un nombre desconocido se muestra como `Reset`.

---

## 🌐 3. Uso de Pairee sobre SSH y Teclas Modificadoras (Ctrl / Alt)

Al usar **Pairee** de forma remota a través de conexiones SSH, notarás que mantener presionados los botones físicos `Ctrl` o `Alt` no actualiza de forma automática la barra inferior de teclas F. Esta es una limitación inherente al protocolo SSH y a los terminales estándar, que solo envían secuencias de bytes cuando se realiza una combinación completa (no envían eventos cuando los modificadores se presionan o liberan por separado).

Para solventar esta limitación, se han implementado diferentes alternativas:

### 3.1 Rotación Manual de Modificadores (Sin Software de Terceros)
Puedes presionar **`Ctrl+Alt+k`** (acción "Ciclar modificadores de la barra de teclas", `cycle_fkeys_modifiers`; asignada en el preset Norton y en los presets Neovim y yazi, que heredan sus teclas, y sin tecla en Standard, donde puedes asignarla en la lista de atajos) para rotar visualmente los estados de la barra de teclas F inferior (`Ctrl+p` no hace esto: en Norton cambia al panel inactivo, y `Ctrl+P` abre la paleta de comandos):
* **Primera pulsación**: Bloquea la barra en la vista de **CONTROL** (ej. F3: Nombre, F4: Extensión).
* **Segunda pulsación**: Bloquea la barra en la vista de **ALT** (ej. F3: Ver, F4: Editar).
* **Tercera pulsación**: Bloquea la barra en la vista de **SHIFT** (ej. F1: Comprimir, F6: renombrado múltiple).
* **Cuarta pulsación**: Restaura la vista por defecto de la barra.

Las filas del panel muestran siempre lo que el mapa de teclas activo asigna a `F1`…`F12` con cada modificador.

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
Puedes acceder al gestor de plugins seleccionando **Barra de menú → Archivos → Comandos plugin**. La tecla `F11` ya no está enlazada a esta acción en el mapa de teclas por defecto.

La ventana emergente presenta dos pestañas principales entre las que puedes cambiar presionando la tecla **`Tab`**:

---

#### Pestaña 1: Plugins Instalados
Muestra una lista con todos los plugins que se han cargado y están instalados localmente.

##### Indicadores de Estado (Badges):
* **`[P]` (Pinned):** El plugin está anclado a su versión actual. No se actualizará de manera automática al ejecutar actualizaciones globales.
* **`[T]` (Trusted):** El plugin tiene permisos de ejecución extendidos ("Modo de Confianza"). Los plugins que no muestran esta etiqueta se ejecutan en modo estrictamente restringido (sandbox), sin acceso a la red ni a comandos arbitrarios del sistema operativo.
* **`[-]` (Desactivado):** El plugin está instalado pero no se carga; no se ejecuta nada de su código ni de sus hooks.
* **`[▲]` (Update Available):** Indica que existe una versión más reciente de este plugin disponible en el repositorio central.

##### Controles de Teclado:
* **`c` / `C` / `Enter`:** Edita los ajustes del plugin seleccionado: un formulario con una casilla o un campo por ajuste (Espacio alterna, Enter guarda, Esc cancela; Predeterminados restaura los valores del plugin). Los valores se comprueban contra su tipo y se aplican de inmediato.
* **`t` / `T`:** Cambia (activa/desactiva) el estado de confianza (Trusted) del plugin seleccionado en la configuración.
* **`e` / `E`:** Activa o desactiva el plugin seleccionado.
* **`s` / `S`:** Activa o desactiva el Modo Seguro global para todos los plugins.

Los cambios de confianza, activación y Modo Seguro se guardan y se aplican al instante: Pairee recarga sus plugins con la nueva configuración. Si `config.toml` no se puede leer, los plugins se ejecutan en Modo Seguro hasta que se corrija. Las descargas del registro se comprueban con las firmas minisign del registro; mientras el registro todavía no esté firmado, las instalaciones y actualizaciones muestran un aviso de "sin firmar".
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
Si deseas escribir tus propios complementos, auditar la arquitectura de seguridad o colaborar con el registro oficial, te invitamos a habilitar el **Modo Desarrollador** en el panel de Configuración (F9 -> Pestaña 4: Idioma y Plugins). Esto activará la tercera pestaña (**Herramientas de Desarrollo / Developer Tools**) en el Gestor de Plugins y te permitirá configurar tu propio directorio de desarrollo personalizado.

#### Herramientas de Desarrollo Disponibles:
* **Seleccionar Plugin Activo (Opción 0):** Abre un modal interactivo con la lista de plugins detectados en el sistema. Escanea tanto la carpeta de desarrollo (`plugins_dev_dir`) como las rutas actuales de los paneles activos (Panel 1 y Panel 2) en busca de un archivo `manifest.toml`, permitiéndote elegir con facilidad el plugin sobre el que deseas trabajar.
* **Inicializar plantilla (Opción 1):** Crea un esqueleto de plugin en tu directorio de desarrollo con los archivos mínimos necesarios (`manifest.toml`, `main.lua` y traducciones en `lang/en.toml`). Al crearlo con éxito, se selecciona automáticamente en la Opción 0 y esta opción se deshabilita.
* **Verificar (Lint) (Opción 2):** Escanea la carpeta del plugin seleccionado y realiza auditorías de seguridad e integridad sobre los archivos Lua y de manifiesto. Detecta llamadas a funciones potencialmente peligrosas (como ejecución de subprocesos externos u operaciones de red no declaradas).
* **Empaquetar (Opción 3):** Prepara un clon local temporal de la rama oficial `plugin-registry`, valida los archivos de metadatos, copia todos los componentes del plugin a la rama, añade/actualiza su entrada de catálogo en el archivo maestro `registry/index.toml` (solo metadata) e incrusta la lista de archivos con sus hashes SHA-256 en la copia de su manifiesto. Si no incluye un archivo de licencia, genera un archivo `LICENSE` MIT estándar y lo asocia automáticamente. Además, muestra claramente en pantalla la ruta del directorio de la caché local donde se empaquetó.
* **Instalar plugin local en desarrollo (Opción 4):** Copia de inmediato tu plugin en desarrollo a la carpeta de ejecución local de Pairee y lo registra en el archivo `plugins.lock`. Esto hace que aparezca al instante en tu lista de "Plugins Instalados" (Pestaña 1) para que puedas probar sus funciones directamente en la interfaz.
* **Enviar plugin (GitHub PR) (Opción 5):** Solicita una descripción del commit e inicia el proceso de envío.
   - **Con Token de GitHub:** Realiza un fork automático del repositorio oficial `FittyAr/Pairee`, sube la rama y abre una Pull Request automáticamente.
   - **Sin Token de GitHub (Manual):** Realiza un commit git local en la carpeta temporal e indica exactamente qué comandos ejecutar en tu terminal para realizar el push y abrir la Pull Request manualmente.
   - **Nota sobre Seguridad:** Si proporcionas el token, este nunca se almacena en el disco ni en variables de entorno persistentes; solo se utiliza de forma transitoria en memoria durante la transacción de red.

Para obtener guías técnicas en profundidad:
* 📚 [**Guía de Desarrollo de Plugins en GitHub**](https://github.com/FittyAr/Pairee/blob/master/docs/plugin-dev-guide.md)
* 🛠️ [**Diseño Técnico de la Arquitectura de Plugins**](https://github.com/FittyAr/Pairee/blob/master/docs/technical/plugin-system-design.md)
* 📂 [**Especificación del Registro Central de Distribución**](https://github.com/FittyAr/Pairee/blob/master/docs/technical/plugin-registry-spec.md)

---

## 🧭 Sesiones y Línea de Comandos

### Restaurar la sesión
Con **Restaurar la última sesión** (`restore_session`, **Opciones → Configuración → Interfaz**, activado por defecto) Pairee guarda la sesión en `session.toml` dentro de la carpeta de configuración al salir y la reabre en el siguiente inicio: todas las pestañas de ambos paneles (carpeta, archivo comprimido, vista, orden, filtro, nombre, bloqueo y el elemento bajo el cursor), la pestaña visible de cada lado, el lado activo, la visibilidad de los paneles y la Vista rápida.

* Una carpeta que ya no existe abre su carpeta padre existente más cercana.
* Las pestañas SFTP se guardan con el nombre de su perfil SSH (nunca la contraseña) y solo se reconectan cuando la pestaña se muestra. Si el perfil ya no existe o la conexión falla, la pestaña queda en tu carpeta personal y un aviso explica el motivo. Las conexiones que no se abrieron desde un perfil se restauran como tu carpeta personal.
* Un `session.toml` dañado se ignora (se guarda una copia como `session.toml.bak` y se registra un aviso).
* **Guardar configuración** solo guarda los valores por defecto (vista, orden, visibilidad de paneles) que se usan cuando no hay sesión que restaurar.

### Argumentos de línea de comandos
```
pairee [--cwd-file <archivo>] [--print-cwd] [--no-mouse] [--standalone] [<carpeta izquierda> [<carpeta derecha>]]
```
* `<carpeta izquierda>` / `<carpeta derecha>` se abren en la pestaña visible de cada panel y tienen prioridad sobre las carpetas restauradas (las demás pestañas se conservan). Un archivo abre su carpeta con el cursor sobre él.
* `--cwd-file <archivo>` escribe al salir la carpeta del panel activo en `<archivo>` (vacío en paneles SFTP), para que una función de la shell haga `cd` allí.
* `--print-cwd` imprime esa carpeta por la salida estándar al salir.
* `--no-mouse` arranca sin capturar el ratón, diga lo que diga la configuración.
* `--standalone` abre Pairee en una ventana de consola nueva y dedicada.
* `--version` (`-V`) muestra la versión instalada y termina.
* `--help` (`-h`) muestra las opciones y los subcomandos (`plugin`, `developer`, `keymap`); `pairee <subcomando> --help` muestra los comandos de cada uno.

### Cambiar la carpeta de la shell al salir
Añade una de estas funciones al perfil de tu shell e inicia Pairee con `p`:

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

---

## 📖 5. Manuales de Integración Avanzada

Para módulos más complejos y detallados, por favor consulta sus manuales específicos:
* **Conexiones remotas (SFTP, FTP, FTPS, WebDAV, S3):** Consulta el [Manual de Conexiones Remotas](ssh_sftp.md).
* **Integración con Git:** Consulta el [Manual de Integración con Git](git_integration.md).
* **Detalle de Ajustes de Configuración:** Consulta el [Manual de Ajustes de Configuración](configuration_details.md).
* **Atajos de Teclado del Sistema:** Consulta la [Guía de Atajos de Teclado](keyboard_shortcuts.md).
* **API Lua de Plugins:** Consulta la [Referencia de la API Lua de Plugins](plugins_es.md) para conocer el namespace `pairee.*` completo (incluyendo los nuevos `pairee.emit`, `pairee.confirm`, `pairee.input`, `pairee.which`, `pairee.notify`, `pairee.file_cache` y `pairee.utils.*` de la evolución M0 del sistema de plugins).

