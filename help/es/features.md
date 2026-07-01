# Manual de Referencia de Funciones de Pairee

Este manual proporciona una descripción detallada de las funciones interactivas, utilidades e integraciones principales disponibles en **Pairee**.

---

## 🖥️ 1. Vistas de Paneles y Diseños Personalizados

Pairee utiliza un diseño clásico de doble panel para la navegación de carpetas y gestión de archivos, permitiendo tener dos directorios a la vista de forma simultánea.

### 1.1 Modos de Visualización del Panel
Puedes configurar cada panel de forma independiente para mostrar archivos usando diferentes niveles de detalle:
* **Brief (Breve):** Muestra solo nombres de archivos en múltiples columnas. Ideal para directorios que contienen miles de archivos.
* **Medium (Medio):** Muestra el nombre y la extensión del archivo lado a lado.
* **Full / Detailed (Detallado):** Muestra metadatos completos del sistema de archivos: Nombre, Extensión, Tamaño, Fecha de Modificación, Permisos (Unix octales), Propietario y conteos de enlaces duros.
* **Wide (Ancho):** Listado de nombres ensanchado con detalles mínimos.
* **Descriptions (Descripciones):** Renderiza el nombre del archivo junto con la descripción cargada desde listas `Descript.ion`.
* **FileOwners:** Lista los archivos junto con los usuarios y grupos.
* **FileLinks:** Muestra los archivos junto con el número de enlaces duros.
* **AltFull:** Estructura de columnas personalizada configurable por el usuario.

### 1.2 Visibilidad e Intercambio de Paneles
* **Alternar Panel Izquierdo/Derecho:** Muestra u oculta de manera individual el panel izquierdo o derecho para centrarse en una sola ruta de directorio.
* **Alternar Ambos Paneles:** Oculta ambos paneles para inspeccionar la salida en consola de comandos en segundo plano o ejecuciones previas.
* **Intercambiar Paneles:** Intercambia instantáneamente las rutas de los paneles izquierdo y derecho.
* **Historial de Navegación:** Muestra una lista de directorios visitados recientemente. Selecciona una fila y presiona `Enter` para saltar directamente a ella.
* **Lista de Favoritos (Hotlist):** Marcadores personalizados para añadir, eliminar y seleccionar tus carpetas más visitadas.

---

## 📂 2. Operaciones del Sistema de Archivos

Las operaciones de archivo en Pairee son asíncronas, procesándose en una cola en segundo plano (`tokio`) para asegurar que la interfaz del usuario permanezca completamente fluida.

### 2.1 Selección Múltiple y Marcado
* Marca archivos pulsando `Insert` o la barra `Espaciadora` sobre un archivo. El cursor se desplaza automáticamente hacia abajo.
* Utiliza la tecla `+` (Teclado numérico) para marcar un grupo de archivos según un patrón de máscara (ej. `*.rs` o `temp_*`).
* Utiliza la tecla `-` (Teclado numérico) para desmarcar archivos que coincidan con el patrón.
* Utiliza la tecla `*` (Teclado numérico) para invertir la selección del panel activo.
* **Filtro del Panel:** Aplica un filtro comodín activo (ej. `*.rs`) para restringir los elementos visibles en el listado del panel actual.

### 2.2 Copiar y Mover/Renombrar
* **Procesamiento en Segundo Plano:** Las tareas de copia y movimiento se ejecutan de forma asíncrona, mostrando barras de progreso en tiempo real, bytes transferidos, nombres de archivos y porcentajes.
* **Resolución de Duplicados:** Si un archivo ya existe en el destino, Pairee te ofrece las opciones de Preguntar (Ask), Sobrescribir (Overwrite), Omitir (Skip) o Añadir (Append).
* **Opciones para Enlaces Simbólicos:**
  - *Smartly copy:* Copia el symlink si el destino lo soporta; de lo contrario, copia su contenido físico.
  - *Copy link:* Copia la referencia del enlace simbólico.
  - *Copy target:* Resuelve el symlink y copia el contenido físico original.

### 2.3 Borrado Seguro (Wipe) y Eliminación
* **Eliminación Normal:** Mueve archivos/carpetas a la papelera del sistema o los borra permanentemente según tu configuración.
* **Borrado Seguro (Wipe):** Sobrescribe los bloques de datos con bytes aleatorios antes de eliminar el archivo físicamente, impidiendo su recuperación mediante herramientas de análisis forense.

### 2.4 Creación de Enlaces
* Crea fácilmente enlaces simbólicos o duros asociando un archivo o carpeta de origen con una ruta de destino específica.

### 2.5 Operaciones con Privilegios Elevados (Administrador / Sudo)
* Cuando una operación (borrado, copia, movimiento o creación de directorio) encuentra un error de "Permiso denegado", Pairee te ofrece la opción de reintentar la acción con privilegios de administrador. La ejecuta usando un ejecutable asistente de elevación (`sudo` en Unix/Linux, solicitud UAC en Windows) sin necesidad de reiniciar la aplicación.

---

## 🔍 3. Búsqueda, Visor y Editor

### 3.1 Búsqueda Avanzada
* **Filtros:** Busca archivos recursivamente con filtros por nombre (ej. `*.toml`, `src*`).
* **Búsqueda por Contenido:** Busca palabras o fragmentos de texto dentro de los archivos.
* **Navegación de Resultados:** La lista de resultados de búsqueda te permite seleccionar cualquier archivo y presionar `Enter` para saltar directamente a él en el panel activo.

### 3.2 Visor Interno y Vista Rápida
* **Modos del Visor:** Alterna entre modo Texto normal y modo Hexadecimal.
* **Modo Hexadecimal:** Muestra offsets, valores hexadecimales y representación ASCII lado a lado. Excelente para inspeccionar archivos binarios.
* **Búsqueda en el Visor:** Presiona `F7` dentro del visor para buscar cadenas de texto.
* **Vista Rápida:** Muestra una vista previa del archivo seleccionado en el panel opuesto. Admite vistas de texto y listado de metadatos de archivos comprimidos.

### 3.3 Editor Interno
* Edita archivos de texto directamente en la aplicación.
* Cuenta con indicadores de línea y carácter actual, junto con advertencias de cambios sin guardar al intentar salir.

---

## 🛠️ 4. Multitarea y Gestión de Pantallas (Screens)

Pairee cuenta con una arquitectura de entornos de trabajo concurrentes (por ejemplo, puedes editar un archivo, ver otro, ejecutar comandos en terminal y explorar los paneles de archivos simultáneamente).

* **Menú de Pantallas:** Muestra la lista de todas las pantallas activas. El entorno actual se marca con un asterisco (`*`).
* **Preservación de Estado (Suspend/Resume):** Cambiar de pantalla mantiene el estado de los diálogos emergentes activos. Por ejemplo, si estás a medio camino en un prompt de copia de archivos, puedes ir a la lista de pantallas, consultar un archivo en el Editor y regresar reanudando la ventana de copia exactamente donde estaba.
* **Atajos de Navegación:** Utiliza los atajos de teclado para avanzar o retroceder en el carrusel de pantallas abiertas sin necesidad de desplegar el menú.

---

## 🧰 5. Utilidades y Herramientas Avanzadas

* **Menú de Acciones Contextuales:** Abre un diálogo contextual con opciones rápidas (Ver, Editar, Copiar, Mover, Eliminar, Comprimir, Extraer) relativas al archivo seleccionado. Detecta archivos comprimidos y añade opciones dinámicas de archivo.
* **Comparar Carpetas:** Analiza y compara las rutas de ambos paneles para resaltar y marcar automáticamente los archivos diferentes, facilitando su sincronización.
* **Administrador de Procesos:** Muestra la lista de procesos activos con sus PIDs, nombres y uso de memoria, permitiendo finalizarlos con `Suprimir` o `Alt+Suprimir`.
* **Vista de Árbol de Directorios:** Recorre la estructura del disco y muestra el árbol de directorios de forma gráfica.
* **Descripciones de Archivos:** Visualiza y edita descripciones presionando `Ctrl+D` sobre cualquier archivo, guardándolas en archivos ocultos `Descript.ion`.
* **Asociaciones de Archivos:** Mapea extensiones de archivos a comandos de ejecución personalizados.
* **Menú de Comandos del Usuario:** Define accesos directos para ejecutar scripts o comandos personalizados sobre los archivos seleccionados.
* **Menú de Selección de Unidad:** Muestra las unidades de almacenamiento locales y de red para cambiar de panel rápidamente.
* **Panel de Información del Sistema:** Ventana que muestra información sobre el sistema operativo, hostname de red, nombre de usuario activo, memoria RAM disponible y variables del sistema.

---

## 🌐 6. Sistema Inteligente de Actualización Automática

Pairee incorpora un sistema de actualización inteligente integrado que identifica cómo se instaló la aplicación y gestiona las nuevas versiones de forma segura y automatizada.

### 6.1 Notificación Interactiva y Ventana Emergente de Versiones
* **Verificación Automática:** Si está habilitada, Pairee realiza una comprobación de la última versión en GitHub Releases de manera asíncrona al arrancar.
* **Indicador de Actualización:** Si existe una nueva versión disponible, se dibuja una etiqueta amarilla `▲ UPDATE` en la barra superior (al lado del reloj).
* **Visor del Registro de Cambios:** Al hacer clic en el indicador o seleccionar `Buscar actualizaciones` en el menú `F9 (Opciones)`, se abre la ventana de Actualización. Este diálogo obtiene y formatea las notas de versión (changelog) directamente desde GitHub y detalla el tamaño de la descarga.

### 6.2 Comportamiento según el Método de Instalación
Pairee analiza 13 métodos de instalación diferentes para aplicar la actualización de forma correcta:
* **Binarios Directos:**
  - **Linux (tar.gz):** Descarga el binario y realiza un reemplazo atómico en la ruta actual de ejecución. Solicita reiniciar para aplicar.
  - **Windows (ZIP):** Descarga la actualización, crea un script `.bat` temporal de auto-eliminación y reemplaza el ejecutable una vez que Pairee se cierra de forma limpia.
  - **Windows (Inno Setup):** Descarga el archivo ejecutable del instalador y lo lanza de forma silenciosa en segundo plano (`/VERYSILENT`).
* **Gestores de Paquetes:** Si detecta que Pairee fue instalado mediante un gestor de paquetes (como `apt`, `dnf`/`rpm`, `pacman`, `nix`, `snap`, `flatpak` en Linux, o `winget`, `scoop`, `chocolatey` en Windows), la ventana mostrará el comando exacto de terminal necesario para actualizar (ej. `winget upgrade Pairee` o `sudo apt update && sudo apt install pairee`) para que puedas ejecutarlo tú mismo en la consola.

### 6.3 Verificación de Firma Segura
Para evitar la ejecución de binarios corruptos o comprometidos, el descargador de Pairee obtiene automáticamente el hash `.sha256` provisto en GitHub Releases y verifica la integridad del archivo descargado antes de proceder con cualquier paso de instalación.

---

## 📖 7. Manuales de Integración Avanzada

Para módulos más complejos y detallados, por favor consulta sus manuales específicos:
* **Conexión SSH y SFTP:** Consulta el [Manual de Conexiones SSH y SFTP](file:///home/fitty/GitHub/Pairee/help/ssh_sftp_es.md).
* **Integración con Git:** Consulta el [Manual de Integración con Git](file:///home/fitty/GitHub/Pairee/help/git_integration_es.md).
* **Detalle de Ajustes de Configuración:** Consulta el [Manual de Ajustes de Configuración](file:///home/fitty/GitHub/Pairee/help/configuration_details_es.md).
* **Atajos de Teclado del Sistema:** Consulta la [Guía de Atajos de Teclado](file:///home/fitty/GitHub/Pairee/help/keyboard_shortcuts_es.md).

