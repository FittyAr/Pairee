# Manual de Ajustes de Configuración

Este manual proporciona una descripción exhaustiva de todas las opciones de configuración disponibles en el Diálogo de Configuración de Pairee (`F2 -> Opciones -> Configuración` o `Comandos -> Configuración`).

> **Nota:** solo se ofrecen las opciones que tienen efecto. Las opciones de versiones anteriores que nunca hicieron nada (por ejemplo descripciones de archivos, formatos de nombre del panel de información, indicadores de compatibilidad del gestor de plugins, páginas de códigos del editor o el comando de editor externo) se eliminaron; si tu `config.toml` todavía las contiene, se ignoran.

---

## 📂 Pestaña 0: Ajustes del Sistema

Esta pestaña controla el procesamiento de archivos, el registro del historial, los permisos de escalado y los criterios de ordenamiento.

### Operaciones de Archivo
* **Delete to Recycle Bin (Eliminar a la Papelera):**
  - *Descripción:* Cuando está habilitado, los archivos eliminados se mueven a la papelera del sistema. Si está desactivado, se eliminan permanentemente.

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
  - *Descripción:* Guarda automáticamente todos los cambios de configuración al salir de Pairee.

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
* **Disable panel update object count (Desactivar recuento de objetos):**
  - *Descripción:* Limita la frecuencia de actualización visual del recuento de archivos para directorios gigantescos, optimizando el rendimiento.
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
* **Mouse support (Soporte de ratón):** Activa interacción por ratón (selección, clics, scrolling).
* **Show bottom F-keys bar (Mostrar barra de teclas F):** Toggles la barra inferior de accesos F1-F10.
* **Always show the menu bar (Mostrar barra de menú siempre):** Mantiene visible el menú superior.

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
* **Modo desarrollador de plugins:** Activa las herramientas de desarrollo de plugins (carpeta de desarrollo y plugin de prueba).

---

## 📂 Pestaña 5: Ajustes del Editor y Visor

### Visor
* **Usar visor externo para F3:** `F3` ejecuta el comando de visualización de la asociación del archivo (ver *Editor de Asociaciones de Archivo* más abajo) y `Alt+F3` abre el visor interno; desactivado es al revés.
* **Usar comando externo al abrir archivos con Enter:** Enter ejecuta el comando de apertura de la asociación en lugar de abrir el visor interno.
* **Tamaño de tabulación / Mostrar barra de desplazamiento:** Ancho de tabulación y barra de desplazamiento del visor interno.

### Editor integrado
Pairee edita los archivos solo con su editor integrado (`F4`); no existe la opción de editor externo.
* **Tamaño de tabulación:** Ancho de una parada de tabulación (2, 4 u 8 columnas).
* **Expandir tabulaciones:** *No expandir tabulaciones* inserta un carácter de tabulación; *Expandir tabulaciones nuevas a espacios* hace que `Tab` inserte espacios hasta la siguiente parada; *Convertir todas las tabulaciones a espacios* además convierte las tabulaciones existentes al abrir el archivo.
* **Auto-sangrado:** `Enter` comienza la nueva línea con la sangría de la actual.
* **Mostrar números de línea:** Muestra la columna de números de línea.
* **Cursor al final:** Abre los archivos con el cursor en la última línea.
* **Bloquear edición de archivos de solo lectura:** Los archivos de solo lectura se abren bloqueados; usa `Shift+F2` para guardar una copia.
* **Avisar al abrir archivos de solo lectura:** Muestra un aviso al abrir un archivo de solo lectura.

---

## 📂 Pestaña 6: Ajustes de Colores

### Configuración del Tema
* **Theme (Tema):** Carga perfiles gráficos (Slate, Blue, High Contrast).
* **Color groups / Highlighting:** Permite personalizar la paleta de colores para los elementos de interfaz y coloreado personalizado de extensiones.

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
