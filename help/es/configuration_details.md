# Manual de Ajustes de Configuración

Este manual proporciona una descripción exhaustiva de todas las opciones de configuración disponibles en el Diálogo de Configuración de Pairee (`F2 -> Opciones -> Configuración` o `Comandos -> Configuración`).

> **Nota:** solo se ofrecen las opciones que tienen efecto. Las opciones de versiones anteriores que nunca hicieron nada (por ejemplo descripciones de archivos, formatos de nombre del panel de información, indicadores de compatibilidad del gestor de plugins, páginas de códigos del editor o el comando de editor externo) se eliminaron; si tu `config.toml` todavía las contiene, se ignoran.

---

## 📂 Pestaña 0: Ajustes del Sistema

Esta pestaña controla el procesamiento de archivos, el registro del historial, los permisos de escalado y los criterios de ordenamiento.

### Operaciones de Archivo
* **Delete to Recycle Bin (Eliminar a la Papelera):**
  - *Descripción:* Cuando está habilitado, los archivos eliminados se mueven a la papelera del sistema. Si está desactivado, se eliminan permanentemente.
* **Los archivos sobrescritos van a la papelera:**
  - *Descripción:* Cuando una copia o un movimiento reemplaza un archivo existente, el archivo anterior se envía antes a la papelera, de modo que deshacer la operación lo recupera. Desactívelo para reemplazar los archivos directamente (`transfer_trash_replaced` en `config.toml`; activado por defecto).

### Funciones opcionales
* **SSH / SFTP:** muestra Conectar SSH en los menús de panel.
* **Plugins Lua:** carga plugins al arrancar y muestra Comandos plugin.
* **Vista previa de imágenes:** decodifica PNG/JPEG en F3 y vista rápida.

### Historial
* **Save commands history (Guardar historial de comandos):**
  - *Descripción:* Guarda el historial de la línea de comandos entre diferentes sesiones.
* **Save folders history (Guardar historial de carpetas):**
  - *Descripción:* Guarda los directorios recientemente visitados en ambos paneles de navegación.
* **Save view and edit history (Guardar historial de visor y editor):**
  - *Descripción:* Almacena las rutas de archivos recientemente consultados o editados.

### Entorno y Registro
* **Use Windows registered types (Usar tipos registrados de Windows):**
  - *Descripción:* (Solo Windows) Consulta el registro del shell de Windows para determinar las asociaciones y descripciones por defecto.
* **Automatic update env variables (Actualizar variables de entorno automáticamente):**
  - *Descripción:* Recarga variables del sistema (como PATH) dinámicamente si se detectan cambios.

### Permisos y Elevación
* **Request admin modification (Solicitar admin para modificaciones):**
  - *Descripción:* Solicita automáticamente privilegios de administrador (sudo/UAC) al intentar modificar o renombrar archivos protegidos por el sistema.
* **Request admin reading (Solicitar admin para lectura):**
  - *Descripción:* Solicita elevación de privilegios si intentas leer o abrir archivos protegidos sin permisos de acceso.

### Criterio de Ordenamiento
* **Sorting collation (Colación de orden):**
  - *Opciones:* `< linguistic >` (orden alfabético) o `< natural >` (como linguistic, pero los números dentro de los nombres se comparan numéricamente, igual que *Tratar dígitos como números*).
* **Treat digits as numbers (Tratar dígitos como números):**
  - *Descripción:* Aplica orden natural. Ej. `archivo2` aparecerá antes que `archivo10`.
* **Case sensitive sort (Sensible a mayúsculas/minúsculas):**
  - *Descripción:* Agrupa y ordena los archivos con nombres en mayúsculas por separado de los de minúsculas.
* **Auto save setup (Autoguardar configuración):**
  - *Descripción:* Guarda automáticamente todos los cambios de configuración al salir de Pairee, incluidos los valores por defecto de los paneles (vista, orden, visibilidad) que se usan cuando no hay sesión que restaurar.

---

## 📂 Pestaña 1: Ajustes de Paneles

Controla las columnas del listado, filtros y actualizaciones.

### Visualización y Selección
* **Show hidden and system files (Mostrar archivos ocultos y de sistema):**
  - *Descripción:* Muestra dotfiles (Linux/macOS) y archivos ocultos del sistema operativo.
* **Highlight files (Resaltar archivos):**
  - *Descripción:* Colorea los archivos según la extensión de su tipo de formato.
* **Select folders (Seleccionar carpetas):**
  - *Descripción:* Al marcar grupos de archivos (`+` o `-`), los directorios también coincidirán con los filtros de máscara.

### Ordenación
* **Sort folder names by extension (Ordenar carpetas por extensión):**
  - *Descripción:* Ordena las carpetas basándose en su sufijo de extensión, en lugar de tratarlas como directorios sin extensión.
* **Sort reverse (Orden inverso):**
  - *Descripción:* Invierte la dirección de la ordenación de los listados en los paneles.
* **Show sort mode letter (Mostrar letra de ordenamiento):**
  - *Descripción:* Renderiza una letra identificativa del criterio activo (ej. `n` para Nombre, `s` para Tamaño) en la barra de estado.

### Actualizaciones e Información
* **Actualizar paneles automáticamente al cambiar las carpetas** (`auto_refresh`, activado por defecto):
  - *Descripción:* Se observa la carpeta de la pestaña activa de cada lado; los archivos que otros programas crean, borran, renombran o modifican aparecen solos en una fracción de segundo, manteniendo cursor y selección, junto con las marcas de Git y los tamaños de carpetas actualizados. Una ráfaga de cambios (una copia grande) refresca el panel cada dos segundos aproximadamente en lugar de con cada archivo. Los paneles de archivos comprimidos no se observan (usa `Ctrl+R`); los SFTP solo con la opción de abajo.
* **Intervalo de sondeo en carpetas de red (segundos)** (`auto_refresh_poll_secs`, 3 por defecto):
  - *Descripción:* Las unidades de red (SMB/NFS mapeadas, rutas UNC y WSL) y otros sistemas de archivos sin notificaciones fiables se comprueban con este intervalo (fecha de modificación y número de entradas). El sondeo también es la alternativa cuando una carpeta no se puede observar.
* **Sondear también las carpetas de paneles SFTP** (`auto_refresh_ssh`, desactivado por defecto):
  - *Descripción:* La carpeta de un panel SFTP se comprueba por la conexión (un `stat` y un listado) con el intervalo de abajo y se relee si cambió. Desactivado por defecto porque cada comprobación es un viaje de red por la conexión del panel.
* **Intervalo de sondeo SFTP (segundos)** (`auto_refresh_ssh_poll_secs`, 30 por defecto):
  - *Descripción:* Intervalo entre comprobaciones de una carpeta SFTP (alterna 10, 15, 30, 60, 300).
* **Disable panel update object count (Desactivar recuento de objetos):**
  - *Descripción:* Se aplica literalmente: las carpetas con más objetos que este valor (0 = sin límite) no se releen automáticamente, ni tras las operaciones ni con el refresco automático (no se observan ni se sondean). `Ctrl+R` siempre relee.
* **Show files total information (Mostrar info total de archivos):**
  - *Descripción:* Muestra el número total de bytes y archivos seleccionados en la línea de estado.
* **Show free size (Mostrar espacio libre):**
  - *Descripción:* Imprime el espacio libre disponible en el disco en la cabecera de la ventana.

### Apariencia
* **Show column titles (Mostrar títulos de columnas):**
  - *Descripción:* Renderiza los encabezados (Nombre, Tamaño, Fecha) sobre la lista.
* **Show status line (Mostrar línea de estado):**
  - *Descripción:* Muestra la barra inferior con los datos de selección.
* **Show scrollbar (Mostrar barra de scroll):**
  - *Descripción:* Dibuja barras de desplazamiento vertical a la derecha del panel.
* **Show ".." in root folders (Mostrar ".." en carpetas raíz):**
  - *Descripción:* Permite ver el enlace de retroceso (`..`) incluso en el directorio raíz (ej. `/` o `C:\`).

---

## 📂 Pestaña 2: Ajustes de Interfaz

Configura aspectos visuales, redibujado de terminal y flujos de trabajo rápidos.

### General
* **Clock (Reloj):** Renderiza la hora digital en tiempo real en la esquina superior derecha.
* **Soporte de ratón:** `Automático` (por defecto) captura el ratón cuando el terminal lo reporta (no en la consola VT de Linux ni con `TERM=dumb`), `Activado` captura siempre y `Desactivado` nunca, dejándole al terminal su selección de texto. La variable de entorno `PAIREE_MOUSE=0|1` fuerza la detección y `--no-mouse` desactiva la captura en esa ejecución. La acción **Soltar ratón** (`toggle_mouse_capture`, sin atajo por defecto) le devuelve el ratón al terminal hasta volver a usarla; además, la mayoría de terminales seleccionan texto con `Shift` + arrastre aunque Pairee capture. Las configuraciones viejas con `mouse_support = true/false` siguen funcionando.
  * **Clic derecho en un archivo:** lo marca (y arrastrar marca un rango, como en Norton y Far) o abre el menú contextual.
  * **Intervalo de doble clic (ms):** pausa máxima entre los dos clics de un doble clic (de 250 a 900).
* **Show bottom F-keys bar (Mostrar barra de teclas F):** Toggles la barra inferior de accesos F1-F10.
* **Always show the menu bar (Mostrar barra de menú siempre):** Mantiene visible el menú superior.
* **Restore last session (Restaurar la última sesión):** Guarda al salir las pestañas, carpetas, el lado activo, la visibilidad de los paneles y la Vista rápida en `session.toml` y los reabre al iniciar (las carpetas indicadas en la línea de comandos tienen prioridad). Las pestañas SFTP se reconectan desde su perfil al mostrarse. Activado por defecto.

### Atajos de teclado
* **Preajuste de atajos:** Alterna Norton / Neovim / VS Code. Una línea de estado indica si el preset (más `custom_bindings`) cargó bien.
* **Ver problemas del mapa de teclas:** Lista acordes rechazados, duplicados y avisos. Los alias Far `Gray+` / `Gray-` / `Gray*` se traducen a `Plus` / `-` / `*`.

### Flujo de Trabajo
* **Enable Yazi workflow (Habilitar flujo de trabajo Yazi):**
  - *Descripción:* Activa menús modales rápidos. Al presionar `s` se abre la ventana de ordenación y al presionar `v` la de vista en la parte inferior (solo disponible si la línea de comandos está vacía).

---

## 📂 Pestaña 3: Ajustes de Confirmaciones

Ajusta qué acciones requieren mostrar una ventana de advertencia antes de llevarse a cabo.

### Operaciones de Archivo
* **Confirmar copiar / mover:** Avisa antes de realizar copias o movimientos. Qué ocurre cuando el archivo de destino ya existe lo decide la propia transferencia (ver `transfer_conflict_resolution` en `config.toml`).
* **Confirmar eliminar / eliminar carpetas no vacías:** Avisa antes de borrar archivos o carpetas con contenido.
* **Confirmar interrupción de operaciones:** Avisar antes de cancelar procesos de hilos en segundo plano.

### Confirmaciones Generales
* **Confirmar recargar archivo editado:** Pregunta antes de que `Ctrl+R` en el editor recargue el archivo desde disco y descarte los cambios sin guardar.
* **Confirmar limpiar historial:** Avisa antes de purgar los registros de base de datos.
* **Confirmar salir:** Avisa al usuario antes de cerrar Pairee.

---

## 📂 Pestaña 4: Ajustes de Idioma y Plugins

### Idioma
* **Main language (Idioma principal):** Permite cambiar el archivo de traducción activo leyendo las configuraciones `.toml` de la carpeta `/lang`.

### Plugins
* **Modo Seguro para todos los plugins:** Ejecuta todos los plugins con las restricciones del Modo Seguro (el mismo interruptor que `s` en el Gestor de plugins). Al pulsar Aceptar se guarda y los plugins se recargan en el acto.
* **Modo desarrollador de plugins:** Activa las herramientas de desarrollo de plugins (carpeta de desarrollo y plugin de prueba).

---

## 📂 Pestaña 5: Ajustes del Editor y Visor

### Visor
* **Usar visor externo para F3:** `F3` ejecuta el comando de visualización de la asociación del archivo (ver *Editor de Asociaciones de Archivo* más abajo) y `Alt+F3` abre el visor interno; desactivado es al revés.
* **Usar comando externo al abrir archivos con Enter:** Enter ejecuta el comando de apertura de la asociación en lugar de abrir el visor interno.
* **Tamaño de tabulación / Mostrar barra de desplazamiento:** Ancho de tabulación y barra de desplazamiento del visor interno.
* **Detectar la codificación automáticamente:** Detecta la codificación de los archivos (marca de orden de bytes, UTF-16, UTF-8, páginas de códigos heredadas). Desactivado, se usa la **Codificación predeterminada** para todos los archivos (una marca de orden de bytes coincidente se sigue omitiendo). `F8` en el visor siempre permite cambiarla.
* **Codificación predeterminada:** Codificación usada cuando la detección está desactivada (`viewer_default_codepage`, un nombre como `windows-1252`; Enter recorre la lista).

### Editor integrado
Pairee edita los archivos solo con su editor integrado (`F4`); no existe la opción de editor externo.
* **Tamaño de tabulación:** Ancho de una parada de tabulación (2, 4 u 8 columnas).
* **Expandir tabulaciones:** *No expandir tabulaciones* inserta un carácter de tabulación; *Expandir tabulaciones nuevas a espacios* hace que `Tab` inserte espacios hasta la siguiente parada; *Convertir todas las tabulaciones a espacios* además convierte las tabulaciones existentes al abrir el archivo.
* **Auto-sangrado:** `Enter` comienza la nueva línea con la sangría de la actual.
* **Mostrar números de línea:** Muestra la columna de números de línea.
* **Cursor al final:** Abre los archivos con el cursor en la última línea.
* **Bloquear edición de archivos de solo lectura:** Los archivos de solo lectura se abren bloqueados; usa `Shift+F2` para guardar una copia.
* **Avisar al abrir archivos de solo lectura:** Muestra un aviso al abrir un archivo de solo lectura.
* **Resaltado de sintaxis (editor y visor):** Colorea los archivos de código según el lenguaje (`syntax_highlighting`). Los colores son el grupo `[syntax]` del tema (ver *Color groups*).
* **Resaltar archivos de hasta:** Los archivos más grandes se abren sin resaltado (`syntax_highlighting_max_kb`; Enter recorre 512 KiB, 1, 2, 8 y 32 MiB).

Ambos ajustes se aplican de inmediato a los editores y visores ya abiertos. Tras *Guardar como* con otra extensión, el editor resalta el archivo según el nuevo lenguaje.

### Definiciones de resaltado de sintaxis
Pairee conoce Rust, C, C++, C#, Go, Java, Kotlin, Swift, JavaScript, TypeScript, Python, Ruby, PHP, Lua, scripts de shell (sh, bash, zsh), PowerShell, archivos batch, HTML, CSS, XML, JSON, YAML, TOML, INI, Markdown, SQL, Makefiles, Dockerfiles y diffs. Cada lenguaje es un pequeño archivo TOML. Para agregar un lenguaje, o cambiar uno incorporado, coloque un archivo `<nombre>.toml` en la carpeta `syntax` del directorio de configuración (junto a `config.toml`). Pairee vuelve a leer la carpeta cuando cambia un archivo en ella, o con el comando *Recargar definiciones de resaltado de sintaxis* (paleta de comandos), y vuelve a resaltar los editores y visores abiertos. Un archivo cuyo `name` es el de un lenguaje incorporado lo reemplaza. Un archivo que no se puede leer o cuyos patrones no son válidos se omite y Pairee indica el motivo; si antes funcionaba, se sigue usando su versión anterior hasta que se corrija.

```toml
name = "Notas"                  # se muestra en la línea de estado del editor
extensions = ["note", "notes"]  # sin el punto, en mayúsculas o minúsculas
names = ["NOTES"]               # nombres de archivo completos (opcional)
filename = '^notes[-_]'         # expresión regular sobre el nombre (opcional)
first_line = '^#!.*\bnotes\b'   # expresión regular sobre la primera línea (opcional)
case_insensitive = false        # todos los patrones sin distinguir mayúsculas (opcional)

[[rule]]                        # colorea cada coincidencia de un patrón...
kind = "keyword"
words = ["TODO", "DONE"]        # ...o cualquiera de estas palabras completas

[[rule]]
kind = "function"
pattern = '@(\w+)'
group = 1                       # colorea solo este grupo de la coincidencia (opcional)

[[region]]                      # de `start` a `end`, aun a lo largo de varias líneas
kind = "string"
start = '"'
end = '"'
skip = '\\.'                    # se saltea al buscar el final (opcional)

[[region.rule]]                 # reglas y regiones dentro de la región
kind = "constant"
pattern = '\\.'
```

Las regiones tienen tres opciones más, para lo que los patrones solos no pueden expresar:

```toml
[[region]]                      # un heredoc: termina en la palabra que lo abrió
kind = "string"
start = '<<-?\s*([A-Za-z_]\w*)'
end = '^\s*${1}$'              # ${1}: el texto del grupo 1 del inicio

[[region]]                      # un literal de expresión regular, no una división
kind = "string"
start = '/'
end = '/[a-z]*|$'
after = '(?:^|[(,=:\[!&|?{};]|\breturn)\s*$'   # el texto anterior al inicio debe coincidir

[[region]]                      # el texto interior se resalta como otro lenguaje
kind = "tag"                    # con `include`, colorea solo el inicio y el final
start = '<script\b[^>]*>'
end = '</script\s*>'
include = "JavaScript"          # nombre o extensión de un lenguaje; '${1}' lo toma del inicio
```

* **`${N}` en `end`** se reemplaza, al abrirse la región, por el texto que coincidió con el grupo de captura `N` de `start`, tomado literalmente: heredocs, cadenas raw de Rust (`r#"…"#`), corchetes largos de Lua (`[==[…]==]`), bloques de Markdown con cercas del mismo largo. El grupo debe existir en `start`.
* **`after`** es un patrón que se prueba sobre el texto anterior al inicio en la misma línea (hasta 256 bytes hacia atrás); la región solo empieza donde coincide, por eso suele terminar en `\s*$`.
* **`include`** nombra un lenguaje (por nombre, o si no por extensión o nombre de archivo, sin distinguir mayúsculas) cuyas reglas y regiones se aplican adentro; el `end` de la región la sigue cerrando, deje lo que deje abierto el lenguaje incluido. Con `'${1}'` el lenguaje se toma del inicio (la cadena de información de un bloque de código de Markdown). Si ese lenguaje no existe, se aplican las reglas propias de la región. Los lenguajes se anidan hasta 4 niveles.
* **`host`** (al principio del archivo, p. ej. `host = "HTML"`) hace que los archivos del lenguaje empiecen en otro lenguaje que lo incluye: los archivos PHP son HTML con PHP dentro de `<?php … ?>`.

* **Tipos** (`kind`): son los colores `[syntax]` del tema: `comment`, `string`, `keyword`, `constant`, `function`, `type_name`, `operator` y `tag`.
* **Patrones:** usan la sintaxis del crate `regex` de Rust (sin look-around ni referencias hacia atrás: use `${N}` y `after`, arriba). Escríbalos entre comillas simples (`'...'`) para que las barras invertidas se mantengan tal cual.
* **Reglas:** se aplican en orden: una regla posterior pinta encima de una anterior. Solo ven el texto fuera de las regiones, o dentro de la región a la que pertenecen.
* **Regiones:** tienen prioridad sobre las reglas: donde podrían empezar varias, gana la que empieza antes (la primera declarada si empatan). Use `end = '$'` para una región que termina con la línea (comentarios de línea), y un `end = ''` vacío para un único token que nada puede partir (un literal de carácter como `'"'`).
* **Detección:** un nombre de `names` gana sobre `filename`, que gana sobre la extensión, que gana sobre la primera línea. Sus archivos se prueban antes que los incorporados.
* De una línea muy larga solo se resaltan los primeros 4 KiB.

Las definiciones incorporadas están en la carpeta [`syntax`](https://github.com/FittyAr/Pairee/tree/master/syntax) del código fuente; copie una como punto de partida.

---

## 📂 Pestaña 6: Ajustes de Colores

### Configuración del Tema
* **Theme (Tema):** Carga perfiles gráficos (Slate, Blue, High Contrast).
* **Color groups / Highlighting:** Permite personalizar la paleta de colores para los elementos de interfaz y coloreado personalizado de extensiones. Los grupos `syntax_*` (comment, string, keyword, constant, function, type_name, operator, tag) colorean el resaltado de sintaxis del editor y del visor; en un archivo de tema son la tabla `[syntax]`, y un tema sin ella usa los colores predeterminados.

---

## 📂 Pestaña 7: Ajustes de Git

### General
* **Enable Git integration (Habilitar integración Git):** Activa el gancho (hook) con el panel de Git.

### Identidad del Autor
* **Author name / Author email (Nombre / Correo del autor):** Sobrescribe los datos de usuario al confirmar cambios (commits). Si se deja en blanco, utiliza los datos configurados en el Git del sistema.
* **Max log entries (Límite del historial):** Determina la cantidad máxima de registros leídos en la pestaña de Log.

---

## 🔗 Editor de Asociaciones de Archivo

Las asociaciones de archivos te permiten mapear patrones de nombres de archivos (máscaras glob) a comandos de ejecución personalizados. Este editor está disponible en **Barra de menú superior (F9) → Comandos → Asociación arch.**

### Atajos de Teclado en el Editor
* `↑` / `↓`: Navegar a través de la lista de reglas.
* `A` / `a` / `Insert`: Añadir una nueva regla de asociación. Se te solicitarán secuencialmente los siguientes datos:
  1. **Máscara (Mask):** Patrón glob (ej: `*.rs` o `*.{jpg,png}`).
  2. **Comando Abrir (Open Command):** El comando de terminal que se ejecutará al abrir el archivo (ej: `explorer %f` en Windows o `xdg-open %f` en Linux). El marcador `%f` se sustituye con la ruta del archivo.
  3. **Comando Ver (opcional - View Command):** Comando para el visor de `F3`. Si se deja en blanco, usará el comando de abrir.
* `E` / `e` / `Enter`: Editar la regla seleccionada. Sigue el mismo asistente paso a paso de ingreso de campos.
* `D` / `d` / `Delete`: Eliminar la regla seleccionada de la lista.
* `Esc`: Salir del editor o cancelar la edición actual.

Todos los cambios se guardan automáticamente en tu archivo de configuración `associations.toml`.

---

## ⚙️ Archivo de Configuración (settings.toml)

Algunos parámetros avanzados se pueden configurar directamente dentro del archivo `settings.toml` (ubicado en tu directorio de configuración):

### Ajustes de Actualización Automática
* **`auto_update_check`** (`bool`, por defecto: `true`):
  - *Descripción:* Cuando está activo, Pairee consulta GitHub Releases en segundo plano al arrancar para buscar nuevas versiones.
* **`dismissed_update_version`** (`string`, por defecto: `null` o vacío):
  - *Descripción:* Almacena la etiqueta de versión (ej. `v1.2.3`) de una actualización que el usuario ha descartado o ignorado de forma explícita, evitando futuras ventanas emergentes sobre esa versión específica. Puedes limpiar este valor si deseas volver a recibir avisos sobre esa versión.

### Comparar y Sincronizar Carpetas
* **`compare_mtime_tolerance_secs`** (`entero`, por defecto: `2`):
  - *Descripción:* Diferencia máxima, en segundos, entre las fechas de modificación de dos archivos que comparar y sincronizar carpetas siguen considerando iguales. El valor por defecto de 2 segundos absorbe la granularidad de 2 segundos de las unidades FAT/exFAT; use `0` para exigir coincidencia exacta.
