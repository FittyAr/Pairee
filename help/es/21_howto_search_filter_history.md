# 🔧 How-To: Buscar, Filtrar, Historial y Hotlist

> **Cuadrante: HOW-TO** — *orientado a problemas.*

Cuatro necesidades del día a día comparten esta página: encontrar un
archivo por nombre, encontrar un archivo por contenido, restringir lo
que muestra el panel actual, y saltar a un lugar donde ya estuviste.

---

## 1. Buscar archivos por nombre (Alt+F7)

**Objetivo:** buscar en uno o más directorios archivos cuyo nombre
coincide con un patrón.

1. Apretá `Alt+F7` (o `Commands → Find file`).
2. En el diálogo, configurá:
   - **Search pattern** — un glob como `*.toml`, un substring como
     `cargo`, o un patrón con llaves como `*.{rs,toml}`.
   - **Search in** — el directorio raíz (default: el panel activo).
   - **Include subdirectories** — toggle recursión.
   - **Case sensitive** — toggle.
   - **Search content** — si está tildado, también escanea el
     contenido de los archivos buscando un string literal
     (ver Sección 2).
3. Apretá `Enter`. Los resultados streamean a una lista.
4. En la lista de resultados:
   - `Up` / `Down` para moverte.
   - `Enter` para **saltar** al archivo en el panel activo.
   - `Esc` para cerrar los resultados sin cambiar el panel.

---

## 2. Buscar archivos por contenido (Alt+F7 con content)

**Objetivo:** encontrar cada archivo que contenga un texto dado.

1. Apretá `Alt+F7`.
2. Tildá **Search content**.
3. Escribí el texto literal en el campo **content**. Los globs en este
   punto los matchea el campo **Search pattern**; el campo content es
   un substring literal plano.
4. Apretá `Enter`.
5. Los resultados muestran cada archivo que contiene el string, una
   entrada por match (por línea en algunos casos). Apretá `Enter` en
   una fila para saltar a ella.

> La búsqueda por contenido es **literal** (sin regex). Es lo
> suficientemente rápida para decenas de miles de líneas en un SSD
> local.

---

## 3. Filtrar el panel activo (Ctrl+I)

**Objetivo:** restringir el panel a un glob, de forma persistente.

1. Enfocá el panel que querés filtrar.
2. Apretá `Ctrl+I` (o `Commands → File panel filter`).
3. Escribí un glob (ej. `*.rs`).
4. Apretá `Enter`. El panel ahora muestra solo los items que
   coincidan.

Para limpiar el filtro, apretá `Ctrl+I` de nuevo y mandá un string
vacío (o `*`).

> El filtro sobrevive el refresh del panel y el swap de paneles,
> pero se resetea cuando cambiás manualmente la ruta del panel.

---

## 4. Quick filter (Ctrl+F o f / F)

**Objetivo:** filtrar el panel *mientras tipeás*, sin paso de commit.

1. Apretá `Ctrl+F` (o `f` / `F` desde el panel).
2. El strip de abajo se vuelve un input. Cada carácter que tipeás
   restringe el panel a archivos cuyos nombres contienen el substring.
3. Apretá `Esc` para soltar el filtro y restaurar el listado
   completo.

> Quick filter es **substring** por default. El diálogo completo
> (Ctrl+I) es basado en glob.

---

## 5. Saltar a una entrada del hotlist de directorios (Ctrl+\)

**Objetivo:** marcar una carpeta como favorita, después saltar de
vuelta a ella más tarde.

1. Navegá hasta un directorio.
2. Apretá `Ctrl+\` (o `Commands → Folder shortcuts`).
3. El diálogo lista tus atajos guardados. Usá:
   - `Insert` para **agregar** la ruta del panel actual a la lista.
   - `Enter` para **saltar** a la entrada resaltada.
   - `Delete` para **quitar** la entrada resaltada.
   - `e` para **renombrar** la entrada resaltada.

Los atajos se guardan en tu carpeta de config como `hotlist.toml`
(o similar). Persisten entre sesiones.

---

## 6. Carpetas recientes (Alt+F12)

**Objetivo:** reabrir un directorio que visitaste recientemente.

1. Apretá `Alt+F12` (o `Commands → Folders history`).
2. El diálogo lista las rutas recientes, las más nuevas primero.
3. `Enter` para saltar a la ruta resaltada; `Delete` para sacarla del
   historial.

> El historial se registra solo si **Save folders history** está
> habilitado en `Configuration → System`.

---

## 7. Archivos recientes en visor y editor (Alt+F11)

**Objetivo:** reabrir un archivo que viste o editaste recientemente.

1. Apretá `Alt+F11` (o `Commands → File view history`).
2. El diálogo lista los archivos que abriste con `F3` o `F4`, los más
   nuevos primero.
3. `Enter` para verlo de nuevo.

> Controlado por la opción **Save view and edit history** en
> `Configuration → System`.

---

## 8. Historial de línea de comandos (Alt+F8)

**Objetivo:** recordar un comando anterior (ej. una ruta que tipeaste
en la línea de comandos).

1. Apretá `Alt+F8` (o `Commands → History`).
2. Un popup de historial lista tus últimos comandos. `Enter` para
   re-ejecutar, `Esc` para cerrar.

> Controlado por **Save commands history** en
> `Configuration → System`.

---

## 9. Tree view (Alt+F10)

**Objetivo:** navegar la estructura de directorios de la ruta actual
como un grafo.

1. Apretá `Alt+F10` (o `Commands → Tree view`).
2. El overlay del árbol muestra el árbol de directorios de la ruta
   del panel activo. Usá flechas para navegar, `Enter` para bajar, `Esc`
   para cerrar.

> Útil cuando querés una vista visual antes de meterte en una carpeta
> profunda.

---

## A dónde ir ahora

- Referencia de la barra F-key: [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
- Settings que controlan el historial: [`41_reference_configuration`](41_reference_configuration.md)
