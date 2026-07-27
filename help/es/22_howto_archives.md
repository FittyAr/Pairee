# 🔧 How-To: Archivos (Comprimir, Extraer, Inspeccionar)

> **Cuadrante: HOW-TO** — *orientado a problemas.*

Pairee soporta los formatos de archivo más comunes out of the box:

- **Compresión**: `zip`, `tar`, `tar.gz`, `tar.bz2`, `tar.xz`, `7z`.
- **Extracción**: `zip`, `tar`, `tar.gz`, `tar.bz2`, `tar.xz`, `7z`,
  `rar` (listado read-only), `gz`, `bz2`, `xz`.

El engine es async: incluso archivos multi-gigabyte se crean o
extraen en workers de background.

---

## Comprimir archivos en un archivo (Shift+F1)

**Objetivo:** empaquetar los archivos resaltados o tagueados en un
nuevo archivo.

1. Tagueá los archivos (o dejá uno resaltado).
2. Apretá `Shift+F1` (o `Files → Add to archive`).
3. En el diálogo:
   - **Archive name** — escribí el nombre del archivo de salida
     incluyendo la extensión. Pairee detecta el formato por la
     extensión (`.zip` → ZIP, `.7z` → 7z, `.tar.gz` → tar+gzip, etc.).
   - **Format** — confirmá o sobreescribí (ZIP, TAR, TAR.GZ, TAR.BZ2,
     TAR.XZ, 7Z).
   - **Compression level** — Normal, Max, Fast, Store.
   - **Include subdirectories** — toggle empaquetado recursivo.
   - **Password** — opcional, solo ZIP y 7Z.
4. Apretá `Enter`. El popup de progreso muestra la compresión en
   curso.

> La detección de formato por extensión es automática. Si tipeás `foo`
> sin extensión, Pairee usa ZIP por default.

---

## Extraer un archivo (Shift+F2)

**Objetivo:** desempaquetar un archivo en el panel pasivo (o un target
elegido).

1. Resaltá el archivo en el panel activo.
2. Apretá `Shift+F2` (o `Files → Extract files`).
3. En el diálogo:
   - **Extract to** — default a la ruta del panel pasivo; editá si
     querés una subcarpeta.
   - **Overwrite policy** — Always, Ask, Skip existing.
   - **Preserve paths** — mantener la estructura de directorios del
     archivo.
4. Apretá `Enter`. La extracción corre en background.

Para archivos `.tar.gz` / `.tar.bz2` / `.tar.xz` / `.zip` / `.7z`,
Pairee elige el decoder correcto automáticamente.

---

## Comandos de archivo (Shift+F3)

**Objetivo:** inspeccionar o modificar un archivo **sin desempaquetar
todo**.

1. Resaltá un archivo.
2. Apretá `Shift+F3` (o `Files → Archive commands`).
3. Un popup muestra las operaciones disponibles. El menú exacto
   depende del tipo de archivo:

   | # | Opción | Disponible para |
   | --- | --- | --- |
   | 1 | Listar contenidos | todos |
   | 2 | Probar integridad | zip, 7z, tar.* |
   | 3 | Extraer acá | todos |
   | 4 | Extraer al otro panel | todos |
   | 5 | Agregar archivos | zip, 7z, tar.* |
   | 6 | Borrar archivos | zip, 7z, tar.* |

4. Elegí la operación y seguí los prompts.

> Los archivos RAR son read-only: podés listarlos y extraerlos, pero
> no agregarles o borrarles entradas.

---

## Navegar un archivo como una carpeta (Quick View)

**Objetivo:** mirar adentro de un archivo sin desempaquetarlo.

1. Resaltá el archivo en cualquier panel.
2. Apretá `Ctrl+Q` para alternar Quick View en el panel pasivo.
3. El panel pasivo ahora muestra las entradas raíz del archivo.
   Apretá `Enter` para bajar a las subcarpetas (Pairee usa una ruta
   virtual `archivo.zip/ruta/interna`).
4. Apretá `Ctrl+Q` de nuevo para cerrar.

Esta es también la forma en que el **visor interno** (F3) maneja los
archivos: muestra un listado del contenido.

---

## Descompresión de un solo archivo

**Objetivo:** desempaquetar un solo archivo `.gz` / `.bz2` / `.xz` (no
un wrapper tar).

1. Resaltá el archivo.
2. Apretá `Shift+F2`.
3. Pairee detecta la compresión de un solo archivo y pregunta por el
   nombre de salida (default: archivo sin la extensión de compresión).
4. Confirmá. La salida se escribe en background.

---

## Errores comunes

- **Archivos con contraseña**: Pairee pregunta por la contraseña al
  extraer. Si seteás una contraseña durante la compresión, el
  receptor debe ingresar el mismo string.
- **Archivos enormes**: la extracción puede tomar tiempo. El popup
  de progreso muestra la velocidad en curso; podés cambiar de
  pantalla mientras corre.
- **Symlinks dentro de archivos tar**: los archivos tar que
  contienen symlinks se extraen de forma segura (los links se
  recrean, no se siguen ciegamente).

---

## A dónde ir ahora

- Workers de background: [`50_explanation_architecture`](50_explanation_architecture.md)
- Keymap completo: [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
