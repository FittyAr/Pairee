# Manual de Ajustes de Configuración

Este manual proporciona una descripción exhaustiva de todas las opciones de configuración disponibles en el Diálogo de Configuración de Pairee (`F2 -> Opciones -> Configuración` o `Comandos -> Configuración`).

---

## 📂 Pestaña 0: Ajustes del Sistema

Esta pestaña controla el procesamiento de archivos, el registro del historial, los permisos de escalado y los criterios de ordenamiento.

### Operaciones de Archivo
* **Delete to Recycle Bin (Eliminar a la Papelera):**
  - *Descripción:* Cuando está habilitado, los archivos eliminados se mueven a la papelera del sistema. Si está desactivado, se eliminan permanentemente.
* **Use system copy routine (Usar rutina de copia del sistema):**
  - *Descripción:* Delega las operaciones de copiado y movimiento a las APIs nativas del SO. Si se desactiva, Pairee utiliza su motor asíncrono interno optimizado mediante hilos de trabajo (Tokio), permitiendo resoluciones de colisiones avanzadas (sobrescribir, omitir, añadir).
* **Copy files opened for writing (Copiar archivos abiertos para escritura):**
  - *Descripción:* Activa si Pairee debe intentar copiar archivos que están bloqueados o siendo modificados por otros procesos de software.
* **Scan symbolic links (Escanear enlaces simbólicos):**
  - *Descripción:* Sigue la ruta física original de los enlaces simbólicos durante las operaciones de archivos.

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
* **Request admin use additional privileges (Solicitar admin para privilegios adicionales):**
  - *Descripción:* Permite el uso de asistentes de escalado del sistema para operaciones avanzadas.

### Criterio de Ordenamiento
* **Sorting collation (Colación de orden):**
  - *Opciones:* `< linguistic >` (Orden alfabético natural humano, ej. `a` luego `B` luego `c`) o `< binary >` (Comparación binaria por bytes ASCII, ej. `B` antes que `a`).
* **Treat digits as numbers (Tratar dígitos como números):**
  - *Descripción:* Aplica orden natural. Ej. `archivo2` aparecerá antes que `archivo10`.
* **Case sensitive sort (Sensible a mayúsculas/minúsculas):**
  - *Descripción:* Agrupa y ordena los archivos con nombres en mayúsculas por separado de los de minúsculas.
* **Auto save setup (Autoguardar configuración):**
  - *Descripción:* Guarda automáticamente todos los cambios de configuración al salir de Pairee.

---

## 📂 Pestaña 1: Ajustes de Paneles

Controla las columnas del listado, filtros, actualizaciones y descripciones de archivos.

### Visualización y Selección
* **Show hidden and system files (Mostrar archivos ocultos y de sistema):**
  - *Descripción:* Muestra dotfiles (Linux/macOS) y archivos ocultos del sistema operativo.
* **Highlight files (Resaltar archivos):**
  - *Descripción:* Colorea los archivos según la extensión de su tipo de formato.
* **Select folders (Seleccionar carpetas):**
  - *Descripción:* Al marcar grupos de archivos (`+` o `-`), los directorios también coincidirán con los filtros de máscara.
* **Right click selects files (Clic derecho selecciona archivos):**
  - *Descripción:* Permite usar el botón derecho del ratón para marcar/taggear elementos en lote en lugar de abrir menús.

### Ordenación
* **Sort folder names by extension (Ordenar carpetas por extensión):**
  - *Descripción:* Ordena las carpetas basándose en su sufijo de extensión, en lugar de tratarlas como directorios sin extensión.
* **Sort reverse (Orden inverso):**
  - *Descripción:* Invierte la dirección de la ordenación de los listados en los paneles.
* **Show sort mode letter (Mostrar letra de ordenamiento):**
  - *Descripción:* Renderiza una letra identificativa del criterio activo (ej. `n` para Nombre, `s` para Tamaño) en la barra de estado.

### Actualizaciones e Información
* **Disable panel update object count (Desactivar recuento de objetos):**
  - *Descripción:* Limita la frecuencia de actualización visual del recuento de archivos para directorios gigantescos, optimizando el rendimiento.
* **Network drives autorefresh (Autorefresh de unidades de red):**
  - *Descripción:* Vigila y refresca automáticamente los cambios en rutas de red montadas.
* **Detect volume mount points (Detectar puntos de montaje):**
  - *Descripción:* Monitorea los discos montados en el sistema de archivos.
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
* **Show background screens number (Mostrar número de pantallas de fondo):**
  - *Descripción:* Muestra la cantidad de pantallas activas en segundo plano.
* **Show ".." in root folders (Mostrar ".." en carpetas raíz):**
  - *Descripción:* Permite ver el enlace de retroceso (`..`) incluso en el directorio raíz (ej. `/` o `C:\`).

### Panel de Información y Descripciones
* **Formatos de Nombre de Máquina/Usuario:**
  - *Descripción:* Configura cómo se renderizan el hostname y el nombre de usuario activo en el panel de información (`Ctrl+L`).
* **Descripciones de Archivos:**
  - *Descripción:* Define las listas (ej. `Descript.ion`), ocultar archivos de descripción, compatibilidad con colores ANSI y codificación UTF-8.

---

## 📂 Pestaña 2: Ajustes de Interfaz

Configura aspectos visuales, redibujado de terminal y flujos de trabajo rápidos.

### General
* **Clock (Reloj):** Renderiza la hora digital en tiempo real en la esquina superior derecha.
* **Mouse support (Soporte de ratón):** Activa interacción por ratón (selección, clics, scrolling).
* **Show bottom F-keys bar (Mostrar barra de teclas F):** Toggles la barra inferior de accesos F1-F10.
* **Always show the menu bar (Mostrar barra de menú siempre):** Mantiene visible el menú superior.
* **Screen saver minutes (Salvapantallas):** Activa un protector de pantalla tras cierto tiempo de inactividad.

### Indicadores de Progreso
* **Show total copy progress / copying time:** Barra agregada de progreso y tiempos de espera al copiar.
* **Show total delete progress:** Barra de progreso para eliminaciones masivas.

### Terminal y Redibujado
* **Use Ctrl+PgUp to change drive:** Permite rotar de unidad usando `Ctrl+PgUp`/`Ctrl+PgDn`.
* **Use virtual terminal:** (Windows) Activa el procesamiento virtual terminal de la consola.
* **ClearType friendly redraw:** Optimiza el redibujado para evitar distorsiones de fuentes tipográficas en terminales específicas.
* **Window Title Format (Formato del Título):** Define las variables del título del terminal (ej. `%Platform`).

### Flujo de Trabajo
* **Enable Yazi workflow (Habilitar flujo de trabajo Yazi):**
  - *Descripción:* Activa menús modales rápidos. Al presionar `s` se abre la ventana de ordenación y al presionar `v` la de vista en la parte inferior (solo disponible si la línea de comandos está vacía).

---

## 📂 Pestaña 3: Ajustes de Confirmaciones

Ajusta qué acciones requieren mostrar una ventana de advertencia antes de llevarse a cabo.

### Operaciones de Archivo
* **Confirmar copiar / mover / sobrescribir:** Avisa antes de realizar copias, movimientos o reemplazar archivos duplicados en destino.
* **Confirmar drag and drop:** Avisa antes de completar acciones de arrastre con ratón.
* **Confirmar eliminar / eliminar carpetas no vacías:** Avisa antes de borrar archivos o carpetas con contenido.

### Discos y Sistema
* **Confirmar interrupción de operaciones:** Avisar antes de cancelar procesos de hilos en segundo plano.
* **Confirmar desconectar unidad de red / discos subst:** Avisa al desmontar rutas locales/remotas.
* **Confirmar desmontar disco virtual / remoción hotplug:** Avisa al retirar unidades de disco lógicas.

### Confirmaciones Generales
* **Confirmar recargar archivo editado:** Avisa al editor si el archivo abierto ha sido modificado fuera de la aplicación.
* **Confirmar limpiar historial:** Avisa antes de purgar los registros de base de datos.
* **Confirmar salir:** Avisa al usuario antes de cerrar Pairee.

---

## 📂 Pestaña 4: Ajustes de Idioma y Plugins

### Idioma
* **Main language (Idioma principal):** Permite cambiar el archivo de traducción activo leyendo las configuraciones `.toml` de la carpeta `/lang`.

### Plugins
* **Soporte de plugins OEM (OEM plugin support):**
  - *Descripción:* Permite cargar y procesar plugins heredados codificados en formato de consola OEM (ej. CP437, CP850). Convierte dinámicamente sus salidas a UTF-8 para evitar errores de renderizado de caracteres.
* **Escanear enlaces simbólicos (Scan symlinks):**
  - *Descripción:* Determina si el motor de plugins debe seguir y escanear enlaces simbólicos al buscar nuevos complementos en el directorio de plugins.
* **Procesamiento de archivos (File processing):**
  - *Descripción:* Delega la apertura o procesamiento de archivos y extensiones registradas a los plugins correspondientes (ej. para explorar archivos comprimidos como si fuesen carpetas).
* **Mostrar asociación estándar (Show standard association):**
  - *Descripción:* Muestra las aplicaciones predeterminadas del sistema operativo junto con las opciones de plugins al solicitar abrir un archivo con asociaciones múltiples.
* **Incluso si solo se encuentra un plugin (Even if only one plugin is found):**
  - *Descripción:* Muestra el diálogo de confirmación y selección para procesar el archivo incluso si solo existe un único plugin capaz de manejar ese formato (de lo contrario, se procesará directamente sin preguntar).
* **Resultados de búsqueda (SetFindList) (Search results):**
  - *Descripción:* Permite que un plugin intercepte y manipule el listado de resultados de búsqueda (ej. para volcar búsquedas avanzadas directamente a la lista activa del panel).
* **Procesamiento de prefijos (Prefix processing):**
  - *Descripción:* Habilita el reconocimiento y procesamiento de comandos con prefijo (ej. `ftp:servidor` o `arc:archivo.zip`) para invocar directamente a un plugin específico desde la línea de comandos.

---

## 📂 Pestaña 5: Ajustes del Editor y Visor

### Comandos Externos
* **Usar editor externo / Comando de edición:** Redirige la acción de `F4` a un comando personalizado (ej. `nano %f`).
* **Usar visor externo / Comando de visualización:** Redirige la acción de `F3` a un comando de lectura personalizado (ej. `less %f`).

### Editor Interno
* **Tab size (Tamaño de tabulación):** Espaciado del tabulador.
* **Expand tabs (Expandir tabulación):** Convierte tabuladores en espacios simples.
* **Persistent blocks / Del removes blocks:** Criterios de retención de texto seleccionado.
* **Cursor beyond EOL:** Permite situar el cursor libremente después del fin de línea.
* **Show line numbers / scrollbar / whitespace:** Activa visualizadores de formato.

---

## 📂 Pestaña 6: Ajustes de Colores

### Configuración del Tema
* **Theme (Tema):** Carga perfiles gráficos (Slate, Blue, High Contrast).
* **Color groups / Highlighting:** Permite personalizar la paleta de colores para los elementos de interfaz y coloreado personalizado de extensiones.

---

## 📂 Pestaña 7: Ajustes de Git

### General
* **Enable Git integration (Habilitar integración Git):** Activa el gancho (hook) con el panel de Git.
* **Auto-detect git repos (Autodetectar repositorios):** Activa el escaneo en carpetas para identificar repositorios activos.

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
  2. **Comando Abrir (Open Command):** El comando de terminal que se ejecutará al abrir el archivo (ej: `notepad %f` o `code %f`). El marcador `%f` se sustituye con la ruta del archivo.
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

