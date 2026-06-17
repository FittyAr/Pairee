# Manual de Referencia de Funciones de Pairee

Este manual proporciona una descripción exhaustiva de cada función interactiva, utilidad y consola de integración disponible en **Pairee**.

---

## 🖥️ 1. Vistas de Paneles y Diseños Personalizados

Pairee utiliza una arquitectura de doble panel para la navegación de carpetas y administración de archivos.

### 1.1 Modos de Visualización del Panel
Puedes configurar cada panel de forma independiente para mostrar archivos usando diferentes columnas y niveles de detalle:
* **Brief (Breve):** Muestra solo nombres de archivos en múltiples columnas. Ideal para directorios que contienen miles de archivos.
* **Medium (Medio):** Muestra el nombre y la extensión del archivo lado a lado.
* **Full / Detailed (Detallado):** Muestra metadatos completos del sistema de archivos:
  * **Nombre y Extensión:** Identificador completo del archivo.
  * **Tamaño:** Tamaño del archivo en bytes (las carpetas muestran `<DIR>`).
  * **Fecha de Modificación:** Última fecha y hora en que se editó el archivo.
  * **Permisos:** Notaciones octales de estilo Unix (ej. `0755`) o atributos de archivo.
  * **Propietario:** El nombre de usuario o grupo del propietario del archivo.
  * **Enlaces:** El número de enlaces físicos que apuntan al archivo.
* **Wide (Ancho):** Nombre ampliado con detalles básicos a la derecha.
* **Descriptions (Descripciones):** Renderiza el nombre del archivo junto con la descripción cargada desde listas `Descript.ion`.
* **FileOwners:** Lista los archivos con sus propietarios y grupos.
* **FileLinks:** Muestra los archivos junto con el número de enlaces duros.
* **AltFull:** Estructura de columnas personalizada configurable por el usuario.

### 1.2 Visibilidad e Intercambio de Paneles
* **Alternar Panel Izquierdo/Derecho (`Ctrl+F1` / `Ctrl+F2`):** Muestra u oculta de manera individual el panel izquierdo o derecho.
* **Alternar Ambos Paneles (`Ctrl+O`):** Oculta ambos paneles para inspeccionar la salida del terminal de comandos ejecutados en segundo plano.
* **Intercambiar Paneles (`Ctrl+U`):** Intercambia instantáneamente las rutas de los paneles izquierdo y derecho.
* **Historial de Navegación (`Alt+F12`):** Muestra una lista de directorios visitados recientemente. Selecciona una fila y presiona `Enter` para saltar directamente a ese directorio.
* **Lista de Favoritos (Hotlist - `Ctrl+\`):** Un menú de marcadores personalizado para añadir, eliminar y seleccionar tus carpetas más visitadas.

---

## 📂 2. Operaciones del Sistema de Archivos

Las operaciones de disco son asíncronas y se ejecutan en segundo plano sobre una cola administrada por `tokio`.

### 2.1 Selección Múltiple y Marcado
* Marca archivos pulsando `Insert` o la barra `Espaciadora` sobre un archivo. El cursor se desplaza automáticamente hacia abajo.
* Utiliza la tecla `+` (Teclado numérico) para marcar un grupo de archivos según un patrón de máscara (ej. `*.rs` o `temp_*`).
* Utiliza la tecla `-` (Teclado numérico) para desmarcar archivos que coincidan con el patrón.
* Utiliza la tecla `*` (Teclado numérico) para invertir la selección del panel activo.
* **Filtro del Panel (`Alt+F9` / Opciones):** Aplica un filtro comodín activo (ej. `*.rs`) para restringir los elementos visibles en el listado del panel actual.

### 2.2 Copiar y Mover/Renombrar (`F5` y `F6`)
* **Procesamiento en Segundo Plano:** Las tareas de copia y movimiento se ejecutan de forma asíncrona, mostrando barras de progreso en tiempo real, bytes transferidos, nombres de archivos y porcentajes.
* **Resolución de Duplicados:** Si un archivo ya existe en el destino, Pairee te ofrece las opciones:
  * *Ask (Preguntar):* Muestra un cuadro de confirmación antes de sobrescribir.
  * *Overwrite (Sobrescribir):* Reemplaza el archivo del panel destino silenciosamente.
  * *Skip (Omitir):* Omite la copia de ese archivo y continúa con el resto de la lista.
  * *Append (Añadir):* Concatena el contenido del archivo origen al final del archivo destino.
* **Opciones para Enlaces Simbólicos:**
  * *Smartly copy:* Copia el symlink si el destino lo soporta; de lo contrario, copia su contenido físico.
  * *Copy link:* Copia la referencia/puntero del enlace simbólico.
  * *Copy target:* Resuelve el symlink y copia el contenido físico original.

### 2.3 Borrado Seguro (Wipe) y Eliminación
* **Eliminación Normal (`F8`):** Mueve archivos/carpetas a la papelera del sistema o los borra permanentemente según tu configuración.
* **Borrado Seguro (Wipe):** Sobrescribe los bloques de datos con bytes aleatorios antes de eliminar el archivo físicamente, impidiendo su recuperación mediante herramientas de análisis forense.

### 2.4 Creación de Enlaces (`Ctrl+L` / Alt+F6)
* Crea fácilmente enlaces simbólicos o duros asociando un archivo o carpeta de origen con una ruta de destino específica.

---

## 🔍 3. Búsqueda, Visor y Editor

### 3.1 Búsqueda Avanzada (`Alt+F7`)
* **Filtros:** Busca archivos recursivamente con filtros por nombre (ej. `*.toml`, `src*`).
* **Búsqueda por Contenido:** Busca palabras o fragmentos de texto dentro de los archivos.
* **Navegación de Resultados:** La lista de resultados de búsqueda te permite seleccionar cualquier archivo y presionar `Enter` para saltar directamente a él en el panel activo.

### 3.2 Visor Interno (`F3`) y Vista Rápida (`Ctrl+Q`)
* **Modos del Visor:** Alterna entre modo Texto normal y modo Hexadecimal.
* **Modo Hexadecimal:** Muestra offsets, valores hexadecimales y representación ASCII lado a lado. Excelente para inspeccionar archivos binarios.
* **Búsqueda en el Visor:** Presiona `F7` dentro del visor para buscar cadenas de texto.
* **Vista Rápida (`Ctrl+Q`):** Muestra una vista previa del archivo seleccionado en el panel opuesto.
  * Muestra archivos de texto directamente.
  * Muestra metadatos de archivos comprimidos (ZIP, TAR), incluyendo la tasa de compresión y la cantidad de archivos internos.
  * Muestra una advertencia `[Binary file — cannot preview]` para formatos no soportados.

### 3.3 Editor Interno (`F4`)
* Edita archivos de texto directamente en la aplicación.
* Cuenta con indicadores de línea y carácter actual, junto con advertencias de cambios sin guardar al intentar salir.

---

## 🛠️ 4. Multitarea y Gestión de Pantallas (Screens)

Pairee cuenta con una arquitectura robusta de entornos de trabajo concurrentes (por ejemplo, puedes editar un archivo, ver otro, ejecutar comandos en terminal y explorar los paneles de archivos simultáneamente).

### 4.1 Menú de Pantallas (`F2 -> Comandos -> Lista de pantallas`)
* Muestra la lista de todas las pantallas activas. El entorno actual se marca con un asterisco (`*`).
* Selecciona cualquier pantalla y pulsa `Enter` para cambiar a ella al instante.
* **Preservación de Estado (Suspend/Resume):** Cambiar de pantalla mantiene el estado de los diálogos emergentes activos. Por ejemplo, si estás a medio camino en un prompt de copia de archivos, puedes ir a la lista de pantallas, consultar un archivo en el Editor y regresar reanudando la ventana de copia exactamente donde estaba.

### 4.2 Acciones de Siguiente / Anterior Pantalla
* Utiliza los atajos de teclado para avanzar o retroceder en el carrusel de pantallas abiertas sin necesidad de desplegar el menú.

---

## 🧰 5. Utilidades y Herramientas Avanzadas

### 5.1 Menú de Acciones Contextuales
* Abre un cuadro de diálogo contextual con opciones rápidas (Ver, Editar, Copiar, Mover, Eliminar, Comprimir, Extraer) relativas al archivo seleccionado.
* Detecta archivos comprimidos (ZIP, 7z, RAR, TAR, GZ, BZ2, XZ) e incluye de forma automática la opción de extracción en la lista.

### 5.2 Comparar Carpetas
* Analiza y compara las rutas del panel izquierdo y derecho.
* Detecta diferencias de tamaño o fechas de modificación.
* Marca automáticamente los archivos diferentes en el panel activo para facilitar su sincronización.

### 5.3 Administrador de Procesos (`Alt+F9`)
* Muestra la lista de procesos activos con sus PIDs, nombres y uso de memoria.
* Finaliza procesos directamente seleccionándolos y pulsando la tecla `Suprimir` o `Alt+Suprimir`.

### 5.4 Vista de Árbol de Directorios (`Alt+F10` / Botón Tree)
* Recorre la estructura del disco y muestra el árbol de directorios de forma gráfica.
* Navega por el árbol y presiona `Enter` para cambiar la ruta del panel activo al directorio seleccionado. También se puede activar dentro de los prompts de Copiar y Mover para seleccionar gráficamente la ruta destino.

### 5.5 Descripciones de Archivos (`Ctrl+D`)
* Pairee soporta descripciones de archivos a través de ficheros ocultos `Descript.ion` y `Files.bbs`.
* Visualiza y edita descripciones presionando `Ctrl+D` sobre cualquier archivo. Pairee guarda automáticamente los cambios en archivos de descripción ocultos del directorio.

### 5.6 Asociaciones de Archivos
* Mapea extensiones de archivos (ej. `*.py`, `*.rs`) a comandos de ejecución personalizados.
* Configura acciones específicas para abrir (`Enter`) o visualizar (`F3`) ciertos tipos de archivos.

### 5.7 Menú de Comandos del Usuario (`F2 -> Comandos -> Editar menú de usuario`)
* Define accesos directos para ejecutar scripts o comandos personalizados sobre los archivos seleccionados.

### 5.8 Menú de Selección de Unidad (`Alt+F1` / `Alt+F2` / `Ctrl+PgUp`)
* Muestra las unidades de almacenamiento USB, unidades locales y conexiones de red montadas para cambiar de panel rápidamente.

### 5.9 Panel de Información del Sistema (`F2 -> Comandos -> Panel de información`)
* Ventana que muestra información sobre el sistema operativo, hostname de red, nombre de usuario activo, memoria RAM disponible y variables del sistema.

---

## 🌐 6. Cliente SSH y SFTP Integrado

Pairee incluye un cliente SSH y un motor de protocolo SFTP completamente integrado, permitiéndote administrar archivos en servidores remotos Unix/Linux o Windows a través de SSH de la misma forma que si fuesen directorios locales.

### 6.1 Opciones de Conexión Multi-Modo
* **Marcadores y Ajustes Preestablecidos (Presets):** Guarda los detalles de conexión (nombre, host, puerto, usuario, ruta del archivo de clave) como presets en la configuración para conectarte con un solo clic en futuras sesiones.
* **Autenticación Flexible:**
  * **Contraseña:** Entrada segura e interactiva de contraseñas.
  * **Llave Privada:** Admite el uso de llaves estándar (RSA, Ed25519, etc.), con soporte opcional para frase de paso (passphrase).
  * **Detección Automática de Llaves:** Busca y prueba automáticamente las rutas de llaves predeterminadas como `~/.ssh/id_rsa` o `~/.ssh/id_ed25519`.
  * **Soporte de SSH Agent:** Aprovecha el SSH Agent activo en el sistema local para una autenticación sin contraseña ni llave explícita.

### 6.2 Paneles Remotos Interactivos
* **Sufijo de Título Dinámico:** Al conectarte, el panel activo actualiza su título a `[SSH: usuario@host]` para dar una visibilidad clara del contexto remoto.
* **Operaciones Remotas de Disco:** Permite listar directorios, crear carpetas nuevas (`F7`), renombrar elementos (`F6`) y eliminar recursivamente archivos y carpetas (`F8`) directamente en el servidor SFTP.
* **Permisos y Atributos de Archivos:** Representa los permisos remotos (notación octal Unix), propietarios, grupos, tamaños y marcas de tiempo de modificación en la vista Detallada (Detailed).

### 6.3 Transferencia Asíncrona Bidireccional
* **Procesamiento en Segundo Plano (Tokio):** Transfiere archivos entre tu máquina local y el servidor remoto en ambas direcciones. Las tareas de Copiar (`F5`) y Mover (`F6`) se ejecutan en segundo plano sin congelar la interfaz, mostrando velocidad de transferencia en tiempo real, porcentajes y barras de progreso.
