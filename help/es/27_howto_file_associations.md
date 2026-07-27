# 🔧 How-To: Asociaciones de Archivos

> **Cuadrante: HOW-TO** — *orientado a problemas.*

Las asociaciones de archivos te dejan mapear una **máscara glob** (ej.
`*.rs`, `*.{jpg,png}`) a un **comando de apertura**. Cuando apretás
`Enter` sobre un archivo cuyo nombre matchea una máscara, Pairee
corre el comando mapeado. También podés definir un **comando de
visualización** separado usado por `F3`.

> Para comandos one-off, usá el diálogo **Apply command** (`Ctrl+G`)
> — mirá [`20_howto_file_operations`](20_howto_file_operations.md).
> Para comportamiento repetible por extensión, usá el editor de File
> Associations.

---

## Abrir el editor

Hay dos formas:

1. **Menú superior**: `F9` → `Commands` → `File associations`.
2. **Línea de comandos** (dentro de Pairee): escribí `associations` y
   apretá `Enter`.

El editor se abre como un popup con una lista de una sola columna con
las reglas. Si no hay reglas, la lista está vacía y el hint de abajo
muestra `[A] Add  [Esc] Close`.

---

## Agregar una nueva asociación

1. Apretá `A`, `a`, o `Insert`.
2. Pairee te lleva por tres campos de input, en orden:

   | Paso | Campo | Ejemplo |
   | --- | --- | --- |
   | 1 | **Mask** | `*.rs` o `*.{jpg,png,jpeg}` |
   | 2 | **Open command** | `code %f` (abrir VS Code sobre el archivo) |
   | 3 | **View command** *(opcional)* | `less %f` (usado por `F3`) |

   - `%f` se reemplaza por la **ruta completa** del archivo.
   - `%p` es el **directorio contenedor** del archivo.
   - `%%` es un `%` literal.

3. Después del último campo, apretá `Enter` para commit. La nueva
   regla aparece en la lista.

> La nueva regla se **persiste inmediatamente** en `associations.toml`
> en tu carpeta de config. No hace falta guardar manualmente.

---

## Editar una asociación existente

1. Resaltá la regla.
2. Apretá `E`, `e`, o `Enter`.
3. Se vuelve a correr la misma secuencia de tres campos, con los
   valores actuales pre-llenados. Editá y `Enter` para commit.

---

## Borrar una asociación

1. Resaltá la regla.
2. Apretá `D`, `d`, o `Delete`.
3. La regla se elimina. (El cambio también se escribe en
   `associations.toml` inmediatamente.)

---

## Referencia de teclado

| Tecla | Efecto |
| --- | --- |
| `Up` / `Down` | Mueve el resaltado. |
| `A` / `a` / `Insert` | Agrega una nueva regla. |
| `E` / `e` / `Enter` | Edita la regla resaltada. |
| `D` / `d` / `Delete` | Borra la regla resaltada. |
| `Esc` | Cierra el editor (o cancela el campo de input actual). |

---

## Recetas comunes

| Objetivo | Mask | Open command |
| --- | --- | --- |
| Abrir archivos `.md` en un pager | `*.md` | `less %f` |
| Abrir `.pdf` en tu visor | `*.pdf` | `zathura %f` *(Linux)* / `start %f` *(Windows)* |
| Abrir imágenes en `feh` | `*.{jpg,jpeg,png,gif,webp}` | `feh %f` |
| Editar archivos de código en tu editor | `*.{rs,go,py,ts,js}` | `code %f` |
| Diff entre dos archivos | `*.{patch,diff}` | `code --diff %f` |

---

## Interacción con plugins

Pairee también consulta los **plugins** para comportamiento tipo
asociación (mirá [`45_reference_plugins_api`](45_reference_plugins_api.md)).
Cuando hay múltiples handlers registrados para la misma máscara,
Pairee muestra un diálogo de selección (a menos que *Show standard
association* esté deshabilitado en `Configuration → Plugins`).

---

## A dónde ir ahora

- Aplicar un comando one-off: [`20_howto_file_operations`](20_howto_file_operations.md) (sección Ctrl+G)
- Cadena de handlers de plugins: [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md)
