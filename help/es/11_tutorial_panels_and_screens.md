# 🎓 Tutorial: Paneles, Modos de Vista y Pantallas

> **Cuadrante: TUTORIAL** — *orientado al aprendizaje, hands-on.*

Este tutorial enseña los tres conceptos espaciales que vas a usar
todos los días: los **dos paneles**, los **nueve modos de vista**, y el
**fondo multi-pantalla**. Dominá estos y dominás el 70% de Pairee.

---

## 1. Los dos paneles

La característica definitoria de Pairee es el layout **doble panel**
heredado de Norton Commander. Ambos paneles muestran un listado de
directorio todo el tiempo; uno está *activo* (borde resaltado, tus
teclas aplican acá), el otro es *pasivo* (muestra adónde va a parar tu
próxima copia/movida).

### ¿Qué panel está activo?

- El panel activo tiene un **borde más grueso / de otro color**.
- El reloj y la barra de menú están arriba; la barra F-key al fondo.
- La ruta del panel activo se muestra en la parte superior del listado.

### Cambiar foco

| Tecla | Efecto |
| --- | --- |
| `Tab` | Alterna foco entre los dos paneles. |
| `Shift+Tab` | Igual, en dirección contraria. |
| `Ctrl+U` | **Swap** de los dos paneles (las rutas se intercambian). |
| `Ctrl+O` | **Ocultar ambos** paneles. Apretá de nuevo para traerlos. Útil para inspeccionar salida de background. |
| `Ctrl+F1` | Alterna visibilidad del panel **izquierdo** solamente. |
| `Ctrl+F2` | Alterna visibilidad del panel **derecho** solamente. |

### ¿Por qué dos paneles?

Porque casi toda operación de archivos es una tarea **origen → destino**,
y los dos paneles hacen que origen y destino estén *visibles al mismo
tiempo*. Por eso `F5` (copia) y `F6` (mueve) actúan sobre el panel
activo **hacia** el panel pasivo — no hace falta un picker de destino
separado.

---

## 2. Los nueve modos de vista

Cada panel puede renderizar el mismo directorio de nueve formas
distintas. Elegí la que coincida con la tarea en cuestión.

| `Ctrl+N` | Modo | Qué muestra |
| --- | --- | --- |
| `Ctrl+1` | **Brief** | Solo nombres, en múltiples columnas. Mejor para miles de archivos. |
| `Ctrl+2` | **Medium** | Nombre y extensión, lado a lado. |
| `Ctrl+3` | **Full** | Nombre, tamaño, fecha de modificación. El default del día a día. |
| `Ctrl+4` | **Wide** | Listado ancho de una columna, más caracteres por nombre. |
| `Ctrl+5` | **Detailed** | Permisos Unix, owner, grupo, hardlinks, tamaño real. |
| `Ctrl+6` | **Descriptions** | Nombre + descripción de `Descript.ion` (si existe). |
| `Ctrl+7` | **File Owners** | Nombre + usuario/grupo. |
| `Ctrl+8` | **File Links** | Nombre + conteo de hardlinks. |
| `Ctrl+9` | **Alt Full** | Layout de columnas definido por el usuario. |

> Los nueve modos respetan el mismo filtro y tagueo del panel; solo
> cambia el layout visual.

### Ordenamiento

Las columnas de orden se acceden con **`Ctrl+F3 … Ctrl+F12`**:

| Tecla | Orden |
| --- | --- |
| `Ctrl+F3` | Nombre |
| `Ctrl+F4` | Extensión |
| `Ctrl+F5` | Write time (mtime) |
| `Ctrl+F6` | Tamaño |
| `Ctrl+F7` | Sin orden (orden del filesystem) |
| `Ctrl+F8` | Creation time (birthtime) |
| `Ctrl+F9` | Access time (atime) |
| `Ctrl+F10` | Description |
| `Ctrl+F11` | Owner |
| `Ctrl+F12` | Abre el diálogo Sort Modes (orden compuesto, múltiples columnas) |

Alterná **orden inverso** con `Ctrl+Shift+R` (y ajustá otras opciones
de orden en `Configuration → Panel`).

### Filtrar el panel

Si solo querés ver ciertos archivos, tenés dos herramientas
complementarias:

| Acción | Cuándo usarla |
| --- | --- |
| **File Panel Filter** (`Ctrl+I`) | Persistente: queda activo hasta que lo limpies. Usalo para foco sostenido en un subset. |
| **Quick Filter** (`Ctrl+F` o `f` / `F`) | Vivo, en el momento: en el momento que tipeás, el panel filtra; apretá `Esc` para soltar. |

Ambos aceptan globs (`*.rs`, `*.{toml,yaml}`) y substrings. Mirá
[`21_howto_search_filter_history`](21_howto_search_filter_history.md)
para el set completo de opciones.

---

## 3. El sistema multi-pantalla

Una **pantalla** es una de las cosas que Pairee puede estar "haciendo"
ahora: una vista de panel, un editor, un visor, un popup de atributos,
etc. Podés tener **muchas abiertas al mismo tiempo**, y cambiar entre
ellas sin perder estado.

### Pantallas abiertas

- Apretá `F4` para abrir un archivo de texto en el **editor interno** —
  se crea una nueva pantalla.
- Apretá `F3` para abrir un archivo en el **visor interno** — nueva
  pantalla.
- Apretá `F12` para abrir el **overlay Screens**, que lista cada
  pantalla abierta. La activa está marcada con `*`.

### Cambiar de pantalla

| Tecla | Efecto |
| --- | --- |
| `F12` | Abre el overlay Screens; flechas + `Enter` para saltar. |
| `Ctrl+Tab` | Cicla a la **siguiente** pantalla. |
| `Ctrl+Shift+Tab` | Cicla a la **pantalla anterior**. |
| `Esc` | Cierra el popup actual / desenfoca la línea de comandos. |
| `F10` (o `Ctrl+Q` en algunos presets) | **Quit** Pairee (cierra todo). |

### Preservación de estado

Si iniciás una operación de copia (`F5`), después saltás al editor
para arreglar un typo, después volvés — la copia sigue corriendo y su
popup de progreso está exactamente donde lo dejaste. Las pantallas
**suspenden y reanudan** popups, búsquedas y líneas de comandos, así
que no se pierde trabajo.

### Pantallas y la barra F-key

La barra F-key inferior refleja lo que hay en la **pantalla activa**:

- En una **pantalla de panel**: `1 Help  2 User  3 View  …`
- En la pantalla del **editor**: `1 Help  2 Save  4 Hex  7 Search  8 Discard  10 Quit`
- En la pantalla del **visor**: `1 Help  4 Hex  7 Search  10 Quit`

Siempre podés cerrar la pantalla activa con `F10` / `Ctrl+Q` (preguntará
por confirmación si el editor tiene cambios sin guardar).

---

## 4. Paneles laterales: Quick View e Info

Dos paneles laterales se superponen al panel *pasivo* mientras estén
abiertos:

| Panel lateral | Hotkey | Caso de uso |
| --- | --- | --- |
| **Quick View** | `Ctrl+Q` | Previsualiza instantáneamente el archivo resaltado (texto o listado de archivo) sin abrir una pantalla real. |
| **Info Panel** | `Ctrl+L` | Muestra un overlay de estado con hostname, OS, RAM, ambiente. |
| **Transfer Panel** | `Ctrl+T` | Lista de jobs de transferencia en background, en curso y terminados. |

Apretá la misma tecla de nuevo para cerrar.

---

## 5. Tarjeta de referencia rápida

```
┌───────────────────────┬───────────────────────┐
│  Panel izquierdo (act)│  Panel derecho (pasivo)│
│  /home/me/projects    │  /home/me/backup      │
├───────────────────────┼───────────────────────┤
│ 📁 docs/              │ 📁 docs/              │
│ 📁 src/               │ 📁 notes/             │
│ 📄 README.md          │ 📁 .cache/            │
│ 📄 Cargo.toml         │ 📄 old.zip            │
│ ...                   │ ...                   │
├───────────────────────┴───────────────────────┤
│ 1 Help  2 User  3 View  4 Edit  5 Copy  ...   │
└───────────────────────────────────────────────┘
```

- Apretá `Tab` para mover el borde activo.
- Apretá `Ctrl+U` para swapear las dos rutas.
- Apretá `F12` para administrar muchas pantallas a la vez.

---

## 6. A dónde ir ahora

- Aprendé a **encontrar** cosas: [`21_howto_search_filter_history`](21_howto_search_filter_history.md)
- Aprendé a **editar, copiar, borrar** archivos: [`20_howto_file_operations`](20_howto_file_operations.md)
- Mirá la lista completa de teclas: [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
