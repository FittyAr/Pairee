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
* Marca archivos pulsando `Insert` o la barra `Espaciadora` sobre un archivo. El cursor se desplaza automáticamente hacia abajo. Sobre una carpeta, además mide su tamaño (ver **Tamaño de carpetas** más abajo).
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

### 2.2.1 Renombrado múltiple
Pulsa `Shift+F6` (o **Archivos → Renombrado múltiple**) para renombrar de una vez todos los elementos seleccionados (o el que está bajo el cursor). El diálogo muestra una tabla de vista previa (nombre actual, nombre nuevo, estado) que se actualiza mientras escribes; no se renombra nada hasta pulsar **Renombrar**.
* **Máscaras de nombre y extensión:** `[N]` nombre sin extensión, `[N2-5]` caracteres 2 a 5, `[N3]` el 3.er carácter, `[N2-]` del 2.º al final, `[N2,3]` 3 caracteres desde el 2.º, `[E]` extensión (mismos rangos), `[P]` nombre de la carpeta, `[C]` contador, `[Y]` `[M]` `[D]` `[h]` `[m]` `[s]` fecha y hora de modificación. El resto del texto se copia tal cual; una máscara de extensión vacía elimina el punto.
* **Contador:** valor inicial, paso (puede ser negativo) y número mínimo de dígitos (relleno con ceros).
* **Buscar y reemplazar:** se aplica al nombre nuevo completo; texto literal por defecto o expresión regular (`$1`, `${nombre}` en el reemplazo) si se marca *Expresión regular*. *Ignorar mayúsculas* sirve para ambos.
* **Mayúsculas:** sin cambios, minúsculas, MAYÚSCULAS o Tipo Título, aplicado al final.
* **Detección de conflictos:** las filas con nombre vacío, caracteres que el sistema de archivos no admite (`<>:"/\|?*`, nombres reservados como `CON` en Windows), nombres duplicados o nombres de otros archivos de la carpeta se muestran en rojo y **Renombrar** queda desactivado hasta corregirlos.
* **Ejecución segura:** los renombrados se ordenan para no sobrescribir ningún archivo; los intercambios y ciclos (`a → b`, `b → a`) pasan por un nombre temporal. Si un renombrado falla, se deshacen los ya hechos. Funciona en paneles locales y SSH/SFTP.

### 2.3 Borrado Seguro (Wipe) y Eliminación
* **Eliminación Normal:** Mueve archivos/carpetas a la papelera del sistema o los borra permanentemente según tu configuración.
* **Borrado Seguro (Wipe):** Sobrescribe los bloques de datos con bytes aleatorios antes de eliminar el archivo físicamente, impidiendo su recuperación mediante herramientas de análisis forense.

### 2.4 Creación de Enlaces
* Crea fácilmente enlaces simbólicos o duros asociando un archivo o carpeta de origen con una ruta de destino específica.

### 2.4.1 Deshacer y rehacer operaciones de archivos
`Alt+Retroceso` (o **Archivos → Deshacer**) revierte la última operación de archivos y `Ctrl+Y` (**Archivos → Rehacer**) la repite. El menú muestra qué se va a revertir, por ejemplo *Deshacer: Mover (3 elementos)*. Pairee guarda en memoria las últimas 50 operaciones de la sesión.
* **Qué se puede deshacer:** renombrar y renombrado múltiple (también en paneles SSH/SFTP), mover (`F6`, en la misma unidad o en otra), copiar (se borran las copias nuevas; una copia que sobrescribió un archivo existente nunca se borra), crear carpeta, crear enlace y enviar a la papelera (los elementos se restauran desde la Papelera de reciclaje / papelera en Windows y Linux).
* **Qué no:** el borrado definitivo, la destrucción segura (wipe), copiar/mover/borrar en paneles SSH y enviar a la papelera en macOS. El diario las registra igualmente; al intentar deshacerlas se avisa y se quitan del historial.
* **Comprobaciones de seguridad:** antes de ejecutar nada, una confirmación enumera lo que se va a revertir. Cada elemento se comprueba antes: el archivo debe seguir existiendo con el mismo tamaño y fecha, y su ubicación original debe estar libre. Lo que cambió se muestra como omitido y no se toca; nunca se sobrescribe nada. Deshacer y rehacer usan los mismos caminos que la operación original (trabajos del Transfer Engine, el ejecutor de renombrados), así que el progreso, la cancelación y el registro del trabajo funcionan como siempre.

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
* **Búsqueda en el Visor:** Presiona `F7` dentro del visor para buscar cadenas de texto. La búsqueda recorre todo el archivo en segundo plano (progreso en la línea de estado, `Esc` la cancela).
* **Archivos grandes:** Los archivos se leen por bloques con un índice de líneas construido en segundo plano, así que los de varios gigabytes se abren al instante con memoria acotada.
* **Codificaciones:** La codificación se detecta (marca de orden de bytes, UTF-16, UTF-8, páginas de códigos como Windows-1252 o Shift-JIS) y se muestra en la línea de estado; `F8` permite cambiarla a mano.
* **Vista Rápida:** Muestra una vista previa del archivo seleccionado en el panel opuesto. Admite vistas de texto (primeros 256 KiB, en la codificación detectada) y listado de metadatos de archivos comprimidos.

### 3.3 Editor integrado
* Toda la edición se hace en el editor integrado (`F4`); Pairee nunca lanza un editor externo.
* Deshacer/rehacer (`Ctrl+Z` / `Ctrl+Y`), búsqueda, guardar y guardar como (`Shift+F2`).
* Selección con `Shift` + teclas de movimiento o con el ratón, selección vertical por bloques con `Alt+Shift` + flechas, y copiar / cortar / pegar (`Ctrl+C` / `Ctrl+X` / `Ctrl+V`) con el portapapeles del sistema, o uno interno cuando no hay ninguno disponible.
* Conserva los finales de línea (LF/CRLF), el salto de línea final y el BOM UTF-8; respeta el tamaño de tabulación, la expansión de tabulaciones, el auto-sangrado y los números de línea.
* Avisa o bloquea los archivos de solo lectura y pregunta antes de sobrescribir un archivo modificado por otro programa.

---

## 🛠️ 4. Multitarea y Gestión de Pantallas (Screens)

Pairee cuenta con una arquitectura de entornos de trabajo concurrentes (por ejemplo, puedes editar un archivo, ver otro, ejecutar comandos en terminal y explorar los paneles de archivos simultáneamente).

* **Menú de Pantallas:** Muestra la lista de todas las pantallas activas. El entorno actual se marca con un asterisco (`*`).
* **Preservación de Estado (Suspend/Resume):** Cambiar de pantalla mantiene el estado de los diálogos emergentes activos. Por ejemplo, si estás a medio camino en un prompt de copia de archivos, puedes ir a la lista de pantallas, consultar un archivo en el Editor y regresar reanudando la ventana de copia exactamente donde estaba.
* **Atajos de Navegación:** Utiliza los atajos de teclado para avanzar o retroceder en el carrusel de pantallas abiertas sin necesidad de desplegar el menú.

---

## 🧰 5. Utilidades y Herramientas Avanzadas

* **Menú de Acciones Contextuales:** Abre un diálogo contextual con opciones rápidas (Ver, Editar, Copiar, Mover, Eliminar, Comprimir, Extraer) relativas al archivo seleccionado. Detecta archivos comprimidos y añade opciones dinámicas de archivo.
* **Archivos comprimidos como carpetas:** `Enter` (o `Ctrl+Av Pág`) sobre un zip, tar, tar.gz/tgz o 7z lo abre en el panel como una carpeta; el título muestra la ruta interna (`archivo.zip/carpeta/interna`), las subcarpetas se abren con `Enter` y `..` en la raíz del archivo vuelve a la carpeta que lo contiene, con el cursor sobre él. `F5` copia la selección al otro panel mediante el Transfer Engine, con el mismo extractor seguro que **Extraer** (sin rutas fuera del destino, sin escribir a través de enlaces, sin sobrescribir y con límites de tamaño). Los zip también se pueden modificar: `F5` desde otro panel copia archivos y carpetas dentro del zip, `F7` crea una carpeta y `F8` borra entradas (el archivo se reescribe en un temporal que luego reemplaza al original). Los tar y 7z son de solo lectura; las acciones que un archivo comprimido no admite (editar, renombrar, mover, atributos...) muestran un aviso. Los archivos comprimidos de paneles SSH y los que están dentro de otro archivo no se abren como carpetas.
* **Comparar Carpetas:** (**Comandos → Comparar carpetas**) Compara de forma recursiva y en segundo plano (`Esc` cancela) las carpetas de ambos paneles para resaltar y marcar automáticamente los archivos diferentes; una carpeta presente en ambos lados figura como distinta cuando algo en su interior difiere. Las fechas de modificación que difieren como mucho `compare_mtime_tolerance_secs` (2 s por defecto, la granularidad de FAT) cuentan como iguales; en Windows y macOS los nombres se emparejan sin distinguir mayúsculas. Solo se pueden comparar carpetas locales.
* **Sincronizar Carpetas:** (**Comandos → Sincronizar carpetas**) Sincronización de directorios entre ambos paneles al estilo de Total Commander:
  1. *Opciones:* sentido (`Espacio` alterna) — **Izquierda → Derecha** (copia a la derecha lo nuevo y lo modificado; por defecto no sobrescribe un archivo más nuevo de la derecha), **Derecha → Izquierda**, **Ambos sentidos** (gana el archivo más nuevo; los conflictos con la misma fecha se omiten) o **Espejo Izquierda → Derecha** (además borra lo que solo existe a la derecha); comparar el contenido con el algoritmo de hash de las transferencias (dos archivos del mismo tamaño solo son iguales si su contenido coincide); ignorar archivos ocultos; y una máscara de filtro con la sintaxis del filtro de copia (`*.rs;*.toml` incluye, `!target` excluye, separados por `;`). Intercambie los paneles (`Ctrl+U`) para hacer el espejo en el otro sentido.
  2. *Revisión:* tras la comparación (en segundo plano, `Esc` cancela) se lista cada diferencia con su estado, tamaño y acción prevista, además de los totales de archivos y bytes a copiar o borrar. Teclas: `→`/`>` copiar a la derecha, `←`/`<` copiar a la izquierda, `Supr`/`D` borrar (elementos de un solo lado), `S` omitir, `Espacio` alterna entre las acciones permitidas, `Tab` cambia el sentido (restableciendo las acciones por defecto), `E` muestra u oculta los archivos iguales, `Enter` aplica, `Esc` vuelve a las opciones.
  3. *Aplicar:* los planes que borran algo piden una confirmación explícita (`S`/`Y`/`Enter`, `N`/`Esc`). Las copias y borrados se ejecutan como trabajos normales del motor de transferencias (progreso, pausa, cancelación y registro en el panel de transferencias); las copias sobrescriben el destino y conservan la fecha de modificación del origen, y los borrados siguen el ajuste "borrar a la papelera". Solo se pueden sincronizar carpetas locales.
* **Tamaño de carpetas:** `Espacio` o `F3` sobre una carpeta, o **Comandos → Medir carpetas** (las carpetas marcadas, o todas si no hay ninguna marcada), calcula el tamaño total en segundo plano. La columna de tamaño lo muestra hasta que cambies de carpeta; `Esc` detiene el cálculo. No se siguen los enlaces simbólicos, los enlaces duros se cuentan una sola vez (Linux/macOS) y las carpetas que no se pudieron leer por completo se marcan con `+`. Funciona en paneles locales y SFTP.
* **Vista de uso de disco:** **Comandos → Uso de disco** analiza la carpeta actual (cancelable con `Esc`, con progreso en vivo) y lista su contenido de mayor a menor con porcentaje y barra. `Enter`/`→` abre una subcarpeta, `←`/`Retroceso` vuelve, `Supr`/`F8` elimina el elemento resaltado con la confirmación de borrado habitual y `r`/`F5` vuelve a analizar. El resultado queda en caché: reabrir la vista sobre la misma carpeta es instantáneo.
* **Administrador de Procesos:** Muestra la lista de procesos activos con sus PIDs, nombres y uso de memoria, permitiendo finalizarlos con `Suprimir` o `Alt+Suprimir`.
* **Vista de Árbol de Directorios:** Recorre la estructura del disco y muestra el árbol de directorios de forma gráfica.
* **Descripciones de Archivos:** Visualiza y edita descripciones presionando `Ctrl+D` sobre cualquier archivo, guardándolas en archivos ocultos `Descript.ion`.
* **Asociaciones de Archivos:** Mapea extensiones de archivos a comandos de ejecución personalizados.
* **Menú de Comandos del Usuario:** Define accesos directos para ejecutar scripts o comandos personalizados sobre los archivos seleccionados. La ventana emergente se abre con `F2` e incluye un conjunto predeterminado de accesos directos (Actualizar, Mostrar/ocultar archivos ocultos, Intercambiar paneles, Lista de tareas, Panel de Git, **Crear carpeta**, Filtro rápido, Ayuda, Editar).
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

## 🧩 7. Sistema de Complementos y Herramientas de Desarrollador

Pairee soporta un sistema de complementos basado en Lua que permite extender el gestor de archivos con comandos personalizados, visores de archivos y hooks de ciclo de vida.

### 7.1 Gestor de Complementos

Accede al Gestor de Complementos desde **Barra de menú → Archivos → Comandos plugin**. La tecla `F11` ya no está enlazada a esta acción en el mapa de teclas por defecto. Tiene tres pestañas:

- **Instalados:** Lista todos los complementos cargados con versión, insignia de confianza e indicadores de actualización disponible. Usá `Enter` para alternar confianza/anclado, `D` para desinstalar.
- **Registro:** Busca el registro online de complementos e instala en segundo plano.
- **Herramientas de Desarrollador:** Disponible cuando `plugins_developer_mode = true` en los ajustes. Ofrece el asistente de inicialización, lint, empaquetado y envío.

### 7.2 Inicializar un Nuevo Complemento

En la pestaña **Herramientas de Desarrollador**, seleccioná **Inicializar Nuevo Complemento** y seguí el asistente paso a paso:
1. Ingresá el **nombre** del complemento (usado como nombre de carpeta e identificador en el manifiesto).
2. Ingresá una **descripción** breve.
3. Ingresá el nombre del **autor**.

Pairee clona los archivos de código base desde la rama interna `plugin-template`:

```
mi-complemento.pairee/
├── manifest.toml     ← nombre, descripción y autor pre-completados
├── main.lua          ← punto de entrada Lua listo para ejecutar
├── lang/en.toml      ← claves de traducción en inglés por defecto
├── help/en.md        ← documentación de ayuda para el usuario
├── icon.png          ← icono de marcador de posición 256×256
└── screenshots/
    └── screenshot1.png
```

### 7.3 La Rama `plugin-template`

El contenido de los archivos anteriores proviene de una **rama git huérfana dedicada** (`plugin-template`) en el repositorio de Pairee — nunca se muestra en ningún listado de plugins. Pairee localiza el repositorio local automáticamente recorriendo hacia arriba desde la ruta del binario. También podés configurar `PAIREE_REPO_DIR=/ruta/a/pairee` como variable de entorno para sobrescribir esto.

Si el repositorio no está disponible (ej. instalado como binario independiente), los archivos se generan desde los valores predeterminados integrados como fallback.

### 7.4 Comandos de Herramientas de Desarrollador

| Acción | Descripción |
|--------|-------------|
| **Init** | Crear un nuevo complemento desde la plantilla |
| **Lint** | Verificar todos los plugins en desarrollo buscando errores en el manifiesto y llamadas Lua no seguras |
| **Package** | Escanear archivos, generar hashes SHA-256 y generar entrada de registro |
| **Submit** | Validar, hacer fork del repo de Pairee y preparar una Pull Request |

> **Consejo:** Editá la plantilla para futuros complementos haciendo checkout de la rama `plugin-template`, modificando los archivos y realizando un commit. Los nuevos complementos creados a partir de ese momento usarán tu código base actualizado.

---

## 📖 8. Manuales de Integración Avanzada

Para módulos más complejos y detallados, por favor consultá sus manuales específicos:
* **Conexión SSH y SFTP:** Consulta el [Manual de Conexiones SSH y SFTP](ssh_sftp.md).
* **Integración con Git:** Consulta el [Manual de Integración con Git](git_integration.md).
* **Detalle de Ajustes de Configuración:** Consulta el [Manual de Ajustes de Configuración](configuration_details.md).
* **Atajos de Teclado del Sistema:** Consulta la [Guía de Atajos de Teclado](keyboard_shortcuts.md).
* **Guía para Desarrolladores de Complementos:** Consulta la [Guía para Desarrolladores de Complementos](../../docs/plugin-dev-guide-es.md) y la [API Lua v1](../../docs/api/lua/v1.md).

