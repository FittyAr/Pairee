# 🔧 How-To: Operaciones de Archivos

> **Cuadrante: HOW-TO** — *orientado a problemas, asume que conocés lo básico.*

Esta página es una colección de recetas para las operaciones de archivos
más comunes. Cada receta nombra el objetivo, lista los pasos exactos, y
apunta a edge cases o perillas de configuración que pueden importar.

> Las hotkeys por defecto de abajo vienen del preset de keymap
> **Norton Commander**. Si usás **Neovim** o **VSCode**, mirá
> [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md).

---

## Tag y untag de archivos

**Objetivo:** marcar un conjunto contiguo o no contiguo de elementos
para una operación en bulk.

| Tecla | Efecto |
| --- | --- |
| `Insert` / `Space` | Alterna el tag en el archivo resaltado; el cursor baja. |
| `Gray+` (keypad) | Taguea todos los archivos que coincidan con un glob (`*.log`, `temp_*`). |
| `Gray-` (keypad) | Destaguea los archivos que coincidan con un glob. |
| `Gray*` (keypad) | Invierte la selección del **panel entero**. |
| `Ctrl+M` | Restaura el último snapshot de selección en bulk. |

> Cuando "Select folders" está habilitado en
> `Configuration → Panel`, tu glob también matchea directorios.

---

## Copiar archivos (F5)

**Objetivo:** copiar los elementos resaltados o tagueados del panel
activo al panel pasivo.

1. Navegá el panel **activo** hasta el origen y tagueá los archivos
   (o dejá uno solo resaltado para copiar solo ese).
2. Navegá el panel **pasivo** hasta el destino.
3. Apretá `F5`.
4. Si ya existe un archivo en el destino, elegí una de:
   - **Overwrite** — reemplazar el destino.
   - **Skip** — mantener el destino, continuar con el siguiente archivo.
   - **Append** — concatenar (solo válido para algunos tipos de archivo).
   - **Ask** — Pairee preguntará de nuevo por cada archivo.
5. La copia corre en un worker de background. Un **popup de progreso**
   muestra el archivo actual, velocidad, ETA y total de bytes. Podés
   seguir usando Pairee mientras corre; el popup queda encima.

### Opciones de symlink

Si el origen es un symlink, `F5` abre un diálogo extra con:

- **Smartly copy** — copia el puntero del symlink si el destino lo
  soporta; si no, copia los *datos* del target.
- **Copy link** — copia el puntero del symlink tal cual.
- **Copy target** — resuelve el symlink primero; copia los datos
  detrás de él.

### Opciones avanzadas

Apretá `Tab` mientras el campo de destino está enfocado para revelar:

- Filter (un glob aplicado a los archivos origen)
- Preserve attributes (timestamps, permisos)
- Wipe-and-replace (sobreescritura segura antes de mover)

---

## Mover o renombrar (F6)

**Objetivo:** mover archivos (o renombrar in place) entre paneles.

1. Tagueá los archivos (o dejá uno resaltado).
2. Apretá `F6`.
3. Un diálogo pregunta por la ruta destino. Confirmá.
4. Mismo popup de progreso y manejo de overwrite que `F5`.

`F6` también se puede usar como **rename-and-move** para un solo
archivo: escribí la nueva ruta y Pairee lo mueve a la nueva ubicación
en un solo paso.

---

## Renombrar un solo archivo in place (F7)

**Objetivo:** cambiar solo el nombre (no la ubicación) del archivo
resaltado.

1. Resaltá el archivo.
2. Apretá `F7`.
3. Escribí el nuevo nombre. `Enter` para confirmar, `Esc` para
   cancelar.

> `F7` **no** mueve entre directorios. Usá `F6` para eso.

---

## Borrar (F8 / Delete)

**Objetivo:** eliminar los elementos resaltados o tagueados.

1. Tagueá los archivos.
2. Apretá `F8` (o `Delete`).
3. Pairee muestra un diálogo de confirmación. Aceptá.

### Qué pasa realmente

Pairee sigue la configuración **Delete to Recycle Bin** en
`Configuration → System`:

- **Habilitado** (default): los elementos se mueven a la papelera
  del OS (`shell32` en Windows, `trash-cli` / `gio trash` en Linux).
  Se pueden restaurar desde la papelera del sistema.
- **Deshabilitado**: los elementos se borran permanentemente sin
  recuperación.

De cualquier forma, la operación corre de forma asíncrona.

---

## Wipe seguro (Alt+Delete)

**Objetivo:** sobrescribir los sectores de un archivo con datos
aleatorios antes de borrarlo, para que la recuperación forense sea
imposible.

1. Resaltá el archivo.
2. Apretá `Alt+Delete`.
3. Confirmá. Pairee escribe múltiples pasadas (configurable) de bytes
   aleatorios, después borra el archivo.

> El wipe seguro es **lento** y funciona solo en archivos regulares.
> No puede hacer wipe de un SSD completo por el wear-levelling, pero
> sí previene recuperación casual de los sectores liberados.

---

## Crear una carpeta (MkDir)

**Objetivo:** crear un nuevo directorio dentro del panel activo.

1. Apretá `F9` → `Files` → `Make folder`. O apretá `F2` (User Menu) y
   elegí la entrada **`6` Make folder**.
2. Escribí el nombre. `Enter` para confirmar, `Esc` para cancelar.

> También podés crear una cadena de carpetas escribiendo una ruta con
> `/` como separador; Pairee creará los directorios intermedios.

---

## Crear un link (Alt+F6)

**Objetivo:** crear un symbolic o hard link al archivo o carpeta
resaltado.

1. Resaltá el origen.
2. Apretá `Alt+F6` (o `Files → Link`).
3. Elegí **Symbolic** o **Hard** link.
4. Escribí la ruta destino.
5. Confirmá.

> Los hard links no pueden cruzar fronteras de filesystem ni apuntar
> a directorios. Los symbolic links pueden hacer ambas cosas, pero se
> siguen por defecto en copy/move a menos que elijas lo contrario
> (ver opciones de symlink en F5).

---

## Ver archivo (F3) y View Alternate (Alt+F3)

**Objetivo:** leer un archivo sin editarlo.

- `F3` abre el **visor interno**. Alterná texto/hex con `F4` desde
  adentro del visor. Buscá con `F7`.
- `Alt+F3` abre el **visor alternativo** — igual a `F3` pero arrancando
  en el *otro* modo (texto ↔ hex).

Si `Configuration → Editor/Viewer → Use external viewer` está
configurado, `F3` delega a un comando externo (usá `%f` para la ruta
del archivo).

---

## Editar archivo (F4)

**Objetivo:** modificar un archivo de texto.

1. Resaltá el archivo.
2. Apretá `F4`.
3. El editor interno se abre en una nueva pantalla.

Hotkeys del editor interno:

| Tecla | Efecto |
| --- | --- |
| `F2` | **Guarda** el buffer. |
| `F4` | Alterna modo texto / hex. |
| `F7` | Buscar. |
| `F8` | Descarta cambios (con confirmación). |
| `F10` | Salir (pregunta para guardar si hay cambios). |

Si `Configuration → Editor/Viewer → Use external editor` está
configurado, `F4` delega a un comando externo (usá `%f` para la ruta
del archivo).

---

## Ver y cambiar atributos (Ctrl+A)

**Objetivo:** inspeccionar o cambiar metadata de un archivo.

1. Resaltá el archivo.
2. Apretá `Ctrl+A` (o `Files → File attributes`).
3. Un diálogo muestra:
   - En Unix: permisos (octal + simbólico), owner, grupo, mtime, atime.
   - En Windows: read-only, hidden, archive, system flags, timestamps.
4. Alterná los flags y `Enter` para aplicar.

> El mismo diálogo se usa para setear "Hidden" / "System" en Windows,
> o para chmodear un archivo a `0755` en Linux.

---

## Comparar carpetas (Commands → Compare folders)

**Objetivo:** diff entre los dos paneles.

1. Navegá el panel izquierdo a una carpeta, el derecho a otra.
2. `F9` → `Commands` → `Compare folders`.
3. Un diálogo lista los archivos que están:
   - solo en el izquierdo
   - solo en el derecho
   - mismo nombre, diferente tamaño o mtime
   - idénticos
4. Las diferencias se **taguean automáticamente** en el panel activo,
   así que podés copiarlas con `F5`.

---

## Ejecutar un comando en los archivos seleccionados (Ctrl+G)

**Objetivo:** aplicar el mismo comando de shell a cada archivo
tagueado.

1. Tagueá los archivos.
2. Apretá `Ctrl+G` (o `Files → Apply command`).
3. Escribí una plantilla que incluya `%f` (filename) y/o `%p` (path).
   Ejemplo: `convert %f -resize 50% small_%f` para redimensionar todas
   las imágenes tagueadas.
4. El comando corre una vez por archivo, con el placeholder reemplazado.

> Usá la convención **%f** / **%p** / **%%**. La salida se streamea al
> área de strip del panel mientras corre el comando.

---

## Editar una descripción (Ctrl+Z)

**Objetivo:** agregar una descripción de una línea para el archivo
resaltado.

1. Resaltá el archivo.
2. Apretá `Ctrl+Z` (o `Files → Describe files`).
3. Escribí la descripción. `Enter` para guardar.
4. Pairee la escribe en un archivo `Descript.ion` en el mismo
   directorio. El modo de vista `Ctrl+6` rendeará las descripciones
   al lado de los nombres de archivo.

---

## A dónde ir ahora

- El mecanismo de transferencia async (por qué la UI nunca se congela): [`50_explanation_architecture`](50_explanation_architecture.md)
- Papelera, secure-wipe y elevación admin: [`41_reference_configuration`](41_reference_configuration.md)
