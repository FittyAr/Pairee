# Transcripción Completa de Menús y Funciones (Far Manager / Norton Commander)

Este documento detalla la transcripción completa de las funciones, opciones y atajos de teclado de las capturas de pantalla suministradas, seguido por un desglose detallado de su comportamiento y diseño conceptual.

---

## 1. Barra Inferior de Teclas de Función (F1 - F12) y Línea de Comandos
**Ubicación:** Parte inferior de la pantalla principal (Imagen 1).
**Línea de Comandos:** Muestra el prompt actual (ej. `C:\Program Files (x86)\Far Manager>`) con cursor de texto activo para la ejecución de comandos del sistema.

### Teclas y Funciones de la Barra
| Tecla | Etiqueta | Función | Descripción |
| :--- | :--- | :--- | :--- |
| **F1** | `Help` | Ayuda | Abre una ventana flotante con la ayuda contextual y lista de atajos. |
| **F2** | `UserMn` | Menú de Usuario | Abre el menú de comandos personalizados definidos por el usuario. |
| **F3** | `View` | Ver | Abre el visor interno para ver el contenido del archivo seleccionado. |
| **F4** | `Edit` | Editar | Abre el editor de texto interno/externo para modificar el archivo seleccionado. |
| **F5** | `Copy` | Copiar | Inicia el proceso de copia de los elementos seleccionados hacia el panel opuesto. |
| **F6** | `RenMov` | Renombrar/Mover | Inicia la operación para cambiar el nombre o mover elementos al panel opuesto. |
| **F7** | `MkFold` | Crear Carpeta | Abre un diálogo emergente para ingresar el nombre de un nuevo directorio. |
| **F8** | `Delete` | Eliminar | Elimina los elementos seleccionados (con confirmación de seguridad). |
| **F9** | `ConfMn` | Menú de Configuración | Activa y da foco a la barra de menús principal en la parte superior. |
| **F10** | `Quit` | Salir | Cierra limpiamente la aplicación NCRust tras confirmar o guardar cambios. |
| **F11** | `Plugin` | Menú de Plugins | Abre la lista de comandos y extensiones de plugins cargados. |
| **F12** | `Screen` | Lista de Pantallas | Muestra la lista de pantallas/sesiones activas (para conmutar entre ellas). |

---

## 2. Menú Desplegable: "Left" (Panel Izquierdo)
**Ubicación:** Menú superior, columna "Left" (Imagen 2). Permite configurar los modos de vista y estados del panel izquierdo (y por simetría, del panel derecho mediante el menú "Right").

### Opciones de Visualización del Panel
- **`Brief`** (`Ctrl+1`): Muestra solo los nombres de archivos organizados en múltiples columnas compactas.
- **`Medium`** (`Ctrl+2`): Vista intermedia con el nombre del archivo y atributos o detalles básicos (Marcado por defecto con `√` en la captura).
- **`Full`** (`Ctrl+3`): Muestra el nombre del archivo, tamaño, fecha y hora de modificación.
- **`Wide`** (`Ctrl+4`): Muestra nombres de archivos en formato ancho, maximizando el espacio de caracteres.
- **`Detailed`** (`Ctrl+5`): Vista detallada con información extendida (permisos, propietario, tamaño real, etc.).
- **`Descriptions`** (`Ctrl+6`): Muestra los archivos junto con sus descripciones breves leídas de archivos de metadata (ej. `descript.ion`).
- **`Long descriptions`** (`Ctrl+7`): Muestra las descripciones largas completas asociadas a cada archivo.
- **`File owners`** (`Ctrl+8`): Muestra el propietario (usuario/grupo) del archivo según el sistema de archivos del sistema operativo.
- **`File links`** (`Ctrl+9`): Muestra el número de enlaces duros (hardlinks) y simbólicos hacia el archivo.
- **`Alternative full`** (`Ctrl+0`): Modo de visualización alternativo configurable que muestra columnas personalizadas.

### Paneles Especiales e Información
- **`Info panel`** (`Ctrl+L`): Convierte el panel activo en un panel informativo estático que detalla estadísticas del volumen de disco, memoria física libre y propiedades del elemento actualmente seleccionado en el panel opuesto.
- **`Quick view`** (`Ctrl+Q`): Activa la previsualización rápida. Al navegar por los archivos en el panel activo, el panel opuesto muestra automáticamente el contenido del archivo en tiempo real sin abrir el visor F3 independiente.

### Modos y Operaciones del Panel
- **`Sort modes`** (`Ctrl+F12`): Abre un cuadro de diálogo para cambiar el orden de los archivos (por Nombre, Extensión, Fecha, Tamaño, Sin Ordenar, etc.).
- **`Show long names`** (`Ctrl+N`): Alterna entre mostrar nombres de archivos largos completos o truncados al ancho de la columna (Marcado por defecto con `√` en la captura).
- **`Panel On/Off`** (`Ctrl+F1`): Oculta o muestra el panel izquierdo.
- **`Re-read`** (`Ctrl+R`): Vuelve a escanear el directorio actual desde el disco físico para actualizar la lista de archivos.
- **`Change drive`** (`Alt+F1`): Abre un selector gráfico de unidades de disco (C:, D:, USBs, carpetas raíz) para cambiar la ubicación del panel izquierdo.

---

## 3. Menú Desplegable: "Files" (Operaciones de Archivos)
**Ubicación:** Menú superior, columna "Files" (Imagen 3). Agrupa comandos para la manipulación y selección de archivos y carpetas.

### Operaciones Estándar de Archivos
- **`View`** (`F3`): Ver el contenido del archivo con el visor integrado.
- **`Edit`** (`F4`): Editar el archivo seleccionado en modo texto.
- **`Copy`** (`F5`): Copiar elementos seleccionados.
- **`Rename or move`** (`F6`): Renombrar o mover archivos.
- **`Link`** (`Alt+F6`): Crear un enlace simbólico (symlink) o físico (hardlink) al elemento seleccionado en el panel opuesto.
- **`Make folder`** (`F7`): Crear un nuevo directorio.
- **`Delete`** (`F8`): Eliminar elementos seleccionados.
- **`Wipe`** (`Alt+Del`): Eliminación segura de archivos (escribiendo ceros o patrones aleatorios sobre el sector físico antes de borrar la entrada del sistema de archivos, haciendo que sea irrecuperable).

### Operaciones de Archivos Comprimidos (Compresión)
- **`Add to archive`** (`Shift+F1`): Abre una ventana interactiva para comprimir los elementos seleccionados (formatos ZIP, 7Z, TAR, etc.).
- **`Extract files`** (`Shift+F2`): Extrae los contenidos del archivo comprimido seleccionado hacia el panel de destino.
- **`Archive commands`** (`Shift+F3`): Ejecuta comandos específicos dentro de un archivo comprimido sin extraerlo completamente (como verificar integridad, añadir archivos, eliminar un elemento interno, etc.).

### Acciones de Metadata y Ejecución
- **`File attributes`** (`Ctrl+A`): Permite visualizar y modificar los atributos de archivo (Lectura, Oculto, Sistema, Archivo, fechas de creación/modificación, permisos UNIX).
- **`Apply command`** (`Ctrl+G`): Abre un diálogo para aplicar un comando batch o shell sobre todos los archivos seleccionados de forma secuencial.
- **`Describe files`** (`Ctrl+Z`): Permite añadir o editar manualmente una descripción de texto para el archivo seleccionado, almacenándola en el archivo local de descripciones.

### Acciones de Selección Masiva (Grupo)
- **`Select group`** (`Gray +` / tecla `+` del teclado numérico): Abre un prompt de máscara de archivo (ej. `*.rs`) para marcar masivamente los archivos que coincidan.
- **`Unselect group`** (`Gray -` / tecla `-` del teclado numérico): Desmarca masivamente los archivos que coincidan con la máscara dada.
- **`Invert selection`** (`Gray *` / tecla `*` del teclado numérico): Invierte el estado de selección de todos los archivos del panel (los seleccionados se desmarcan y viceversa).
- **`Restore selection`** (`Ctrl+M`): Restaura el estado de la última selección de archivos masiva realizada antes de la última operación de copia, movimiento o borrado.

---

## 4. Menú Desplegable: "Commands" (Comandos de la Aplicación)
**Ubicación:** Menú superior, columna "Commands" (Imagen 4). Contiene herramientas de búsqueda global, navegación histórica y control avanzado de paneles.

### Búsqueda e Historial
- **`Find file`** (`Alt+F7`): Abre una utilidad de búsqueda global para localizar archivos por nombre y/o contenido de texto dentro de una jerarquía de directorios.
- **`History`** (`Alt+F8`): Muestra el historial de comandos de consola ejecutados en la línea de comandos de la parte inferior para volver a ejecutarlos.
- **`Video mode`** (`Alt+F9`): Configura la resolución de pantalla del terminal (ej. número de filas y columnas del buffer).
- **`File view history`** (`Alt+F11`): Muestra una lista de los últimos archivos que han sido abiertos con el visor F3 o editor F4 para un acceso rápido.
- **`Folders history`** (`Alt+F12`): Muestra una lista de los últimos directorios visitados en los paneles para permitir navegación directa instantánea.

### Control e Integridad de Paneles
- **`Swap panels`** (`Ctrl+U`): Intercambia de lugar los contenidos y estado del panel izquierdo con el derecho.
- **`Panels On/Off`** (`Ctrl+O`): Oculta temporalmente ambos paneles de archivo, revelando la salida completa del terminal del sistema inferior (y los restaura al presionar de nuevo `Ctrl+O`).
- **`Compare folders`**: Compara de forma binaria o por nombre los contenidos de los paneles izquierdo y derecho, seleccionando automáticamente aquellos archivos que sean diferentes o que falten en alguno de los dos lados.

### Herramientas de Configuración y Filtrado
- **`Edit user menu`**: Abre el archivo de configuración del Menú de Usuario (F2) en el editor para permitir la edición directa de los scripts y comandos rápidos.
- **`File associations`**: Permite asociar extensiones de archivo (ej. `.rs`, `.py`) con comandos o visores específicos para cuando se presiona `Enter`.
- **`Folder shortcuts`**: Configura marcadores rápidos vinculados a teclas numéricas (ej. `Ctrl+Alt+1` a `d:\GitHub`) para saltar de inmediato a ubicaciones específicas.
- **`File panel filter`** (`Ctrl+I`): Configura filtros de máscara permanentes para ocultar ciertos tipos de archivos en los listados regulares de los paneles.

### Utilidades del Sistema
- **`Plugin commands`** (`F11`): Ejecuta acciones directamente provistas por plugins de terceros.
- **`Screens list`** (`F12`): Lista las pantallas del sistema de Far Manager para alternar entre buffers independientes.
- **`Task list`** (`Ctrl+W`): Abre un administrador de tareas simple que lista los procesos en ejecución del sistema operativo y permite cerrarlos.
- **`Hotplug devices list`**: Lista los dispositivos USB, discos duros externos y otras unidades extraíbles conectadas en caliente al equipo.

---

## 5. Menú Desplegable: "Options" (Configuración General)
**Ubicación:** Menú superior, columna "Options" (Imagen 5). Controla todas las preferencias, configuración del motor de renderizado del terminal, atajos y personalizaciones del sistema.

### Preferencias del Sistema y Paneles
- **`System settings`**: Opciones de comportamiento general (ej. autoguardado al salir, uso del portapapeles del sistema, modo silencioso).
- **`Panel settings`**: Modos predeterminados de ordenación, comportamiento del doble clic, carga de archivos automática al detectar cambios externos.
- **`Interface settings`**: Personalización visual de los paneles (ej. mostrar bordes de línea simple o doble, barra de scroll visible, reloj en pantalla).
- **`Languages`**: Selección de idioma para la interfaz de usuario (inglés, español, etc.).
- **`Plugins configuration`**: Cuadro de diálogo para configurar las opciones específicas de cada plugin registrado.
- **`Plugins manager settings`**: Control de prioridades, directorios de carga y extensiones asociadas al gestor de plugins principal.
- **`Dialog settings`**: Ajustes de comportamiento de los popups e inputs de texto (ej. autocompletado en prompts, historial en inputs).
- **`Menu settings`**: Ajustes de visualización y comportamiento de los menús deplegables superiores.
- **`Command line settings`**: Ajustes del prompt de terminal (historial, colores, autocompletado persistente).
- **`AutoComplete settings`**: Ajustes detallados del motor de sugerencias inteligentes para la línea de comandos y entradas de diálogo.
- **`InfoPanel settings`**: Determina qué información estadística detallada se dibuja al activar el modo de panel de información (`Ctrl+L`).
- **`Groups of file masks`**: Configura grupos lógicos de extensiones de archivos (ej. `[Archivos Fuente]` conteniendo `*.rs, *.c, *.cpp, *.py`) para simplificar tareas de filtrado y ordenación.

### Configuraciones de Archivos y Descripciones
- **`Confirmations`**: Activa o desactiva las advertencias emergentes antes de realizar ciertas operaciones críticas (Eliminar, Copiar, Sobrescribir, Salir).
- **`File panel modes`**: Editor avanzado para definir y customizar la disposición de columnas para cada uno de los 10 modos de panel (`Brief`, `Full`, etc.).
- **`File descriptions`**: Define las reglas para procesar descripciones de archivos (posición del texto en las columnas, caracteres especiales).
- **`Folder description files`**: Configura los nombres de archivo por defecto que almacenan la metadata de descripción del directorio actual.

### Visor, Editor y Codificación
- **`Viewer settings`**: Ajustes del visor interno F3 (tamaño de tabulación, envoltorio de texto (word-wrap), auto-detección de codificación).
- **`Editor settings`**: Ajustes del editor interno F4 (búsqueda y reemplazo, sangría automática, resaltado de sintaxis, comportamiento de guardado).
- **`Code pages`**: Configura las tablas de conversión de caracteres por defecto para visualizar correctamente archivos de texto codificados en UTF-8, ANSI, OEM, ISO, etc.

### Aspecto Visual y Estilos
- **`Colors`**: Permite redefinir la paleta de colores para cada parte de la interfaz TUI (paneles, menús, popups, barra de estado).
- **`Files highlighting and sort groups`**: Configura las reglas de coloración de nombres de archivo basadas en su extensión o máscara (ej. directorios en azul, ejecutables en verde, archivos comprimidos en rojo).

### Guardar Estado
- **`Save setup`** (`Shift+F9`): Guarda inmediatamente toda la configuración, estado de paneles, historial de comandos e historial de carpetas al archivo de configuración centralizado `settings.toml`.
