# 💡 Explicación: Arquitectura, Transferencias Async y Pantallas

> **Cuadrante: EXPLANATION** — *orientado a entender. Discute un tema; no necesitás leer esto para usar Pairee.*

Esta página es para los curiosos. Describe las partes que se mueven
por debajo — el **runtime asíncrono**, el **motor de transferencia**, y
el **stack de pantallas** — y explica *por qué* Pairee nunca se
congela, incluso cuando copiás una carpeta de 50 GB.

---

## 1. El runtime: `tokio` + `crossterm`

Pairee está construido sobre:

- **`ratatui`** — la capa de rendering TUI.
- **`crossterm`** — el backend de terminal cross-platform (parsing de
  input, alternate screen, raw mode).
- **`tokio`** — el runtime asíncrono. Cada operación larga (copia,
  move, delete, wipe, búsqueda, archivo, git, ssh, chequeo de update,
  carga de plugin) se spawnea como una task de `tokio`.

El main thread **nunca se bloquea en I/O**. Es dueño de la terminal y
del estado de la UI. Los workers de background mandan **eventos de
progreso** por canales `mpsc`; el main thread drena el canal entre
dibujadas.

El beneficio: mientras una copia de 50 GB está en progreso, podés
navegar paneles, editar un archivo, cambiar de pantalla, abrir el
popup de ayuda, o disparar otra transferencia. La UI se mantiene a
60 fps.

---

## 2. El motor de transferencia

Ubicado bajo `src/fs/transfer/`, el motor de transferencia es un
pipeline pequeño:

```
        ┌─────────────┐
src ──▶ │   filter    │──▶  pre-condiciones: glob, política de overwrite
        └─────────────┘
                │
                ▼
        ┌─────────────┐
        │   pipeline  │──▶  los bytes fluyen por I/O bufferada
        └─────────────┘
                │
                ▼
        ┌─────────────┐
        │  metadata   │──▶  ownership, mtime, xattrs
        └─────────────┘
                │
                ▼
        ┌─────────────┐
        │ post_action │──▶  hook para plugins / shell-out
        └─────────────┘
                │
                ▼
              dst
```

### Componentes

- **`filter.rs`** — aplica los globs del usuario y las políticas de
  overwrite.
- **`pipeline.rs`** — streamea los bytes en chunks (default 256 KiB)
  por un par reader/writer async. El default es el pipeline puro
  Rust. Seteando `transfer_engine = "direct"` en `settings.toml` se
  cambia a la API de copia del OS (más rápida en algunas plataformas,
  pero sobreescribe sin enforcement de política).
- **`metadata.rs`** — preserva permisos Unix, timestamps, y extended
  attributes cuando se setea `preserve = true`.
- **`hash/`** — verifica integridad. SHA-256, SHA-1, BLAKE3, MD5,
  CRC32 están todos disponibles. Usado para el chequeo de firma del
  sistema de update, la verificación del secure-wipe, y la key del
  file-cache.
- **`engine.rs`** — el orquestador. Tiene la cola de jobs, maneja la
  concurrencia de workers, y emite eventos a la UI.
- **`events.rs`** — los tipos de mensaje. `Started`, `Progress`,
  `Completed`, `Failed`, `Cancelled`.

### Resolución de conflictos

Cuando ya existe un destino, el motor consulta la **política de
conflicto** en este orden:

1. El argumento `overwrite` por operación (diálogo Copy / Move).
2. El setting `Configuration → Confirmations → Confirm overwrite`.
3. El default — **prompt** al usuario.

El diálogo de prompt ofrece:

| Botón | Resultado |
| --- | --- |
| **Overwrite** | Reemplaza el archivo destino. |
| **Overwrite all** | Reemplaza todos los conflictos siguientes. |
| **Skip** | Mantiene el destino, continúa con el siguiente archivo. |
| **Skip all** | Saltea todos los conflictos siguientes. |
| **Append** | Concatena (donde tiene sentido). |
| **Cancel** | Aborta el job entero. |

### Cancelación

El motor vigila un `CancellationToken` por job. Los botones de UI
("Cancel" en el popup de progreso) flipean el token; los workers lo
polean en cada chunk y abortan limpiamente. Los bytes ya copiados
**no** se rollean back, pero el motor escribe un registro
`cancelled` en el historial de transferencias para que sepas qué pasó.

### Concurrencia

Múltiples jobs pueden correr en paralelo. El número de workers
concurrentes default es `min(num_cpus, 4)` y se puede tunear por
operación. El **Transfer Panel** (`Ctrl+T`) muestra el estado en vivo
de cada job.

---

## 3. El stack de pantallas

Una "pantalla" en Pairee es una de las cosas que la UI puede renderear
arriba: una **pantalla de panel**, el **editor**, el **visor**, el
**dashboard Git**, el **overlay de pantallas**, un **diálogo**, etc.

Las pantallas se organizan como un **stack**:

```
┌───────────────────────────────┐
│ Pantalla N (tope)             │  ← activa
├───────────────────────────────┤
│ ...                           │
├───────────────────────────────┤
│ Pantalla 0 (fondo)            │  ← la primera vista de panel
└───────────────────────────────┘
```

Cuando apretás `F4` en un archivo, una nueva pantalla de editor se
**pushea**. Cuando apretás `F10` para salir del editor, se **popea**
y la pantalla de panel de abajo se vuelve activa de nuevo. El estado
(cursor, selección, buffer) se preserva.

### Suspender popups

Una feature sutil: cuando un popup está abierto (digamos, el diálogo
de destino de copia) y apretás `F12` para abrir el overlay de
Pantallas, el diálogo se **suspende** (su estado se guarda) mientras
interactuás con el overlay. Cuando volvés, el diálogo está exactamente
donde lo dejaste.

Esto hace posible arrancar una copia, saltar al editor para arreglar
un typo, saltar al visor para confirmar un archivo, y después volver
y confirmar la copia — todo sin perder el prompt.

### La barra F-key

La barra F-key inferior refleja la **pantalla superior**:

- En una pantalla de **panel**: F1 Help, F2 User, F3 View, F4 Edit, …
- En la pantalla del **editor**: F1 Help, F2 Save, F4 Hex, F7 Search,
  F8 Discard, F10 Quit.
- En la pantalla del **visor**: F1 Help, F4 Hex, F7 Search, F10 Quit.

La barra es puramente visual; los keybindings reales funcionan sin
importar la fila mostrada. (Las terminales SSH que no reportan el
estado de los modificadores pueden usar `Ctrl+P` para lockear la
barra en la fila que quieran — mirá
[`30_howto_ssh_modifier_keys`](30_howto_ssh_modifier_keys.md).)

---

## 4. Por qué esto te importa a vos (el usuario)

La arquitectura tiene tres consecuencias directas que vas a sentir:

1. **La UI nunca se congela.** Si algo parece trabado, la causa más
   probable es la terminal (probá `Ctrl+L` para forzar un redraw) o
   un mount de red (probá `Ctrl+R` para refrescar solo el panel
   activo).

2. **Podés encolar muchas operaciones.** Apretá `F5` en una carpeta,
   después `F5` en otra, después `F5` en una tercera. Todas corren en
   paralelo, y el Transfer Panel trackea cada una independientemente.

3. **La cancelación es barata y segura.** Podés abortar un job largo
   en cualquier momento. El motor se detiene en el próximo boundary
   de chunk, escribe un registro de cancel, y podés limpiar los
   archivos parciales con un delete normal.

---

## 5. Dónde mirar en el código

Si sos un desarrollador curioso del código:

- `src/fs/transfer/mod.rs` — la entry point del motor.
- `src/fs/transfer/pipeline.rs` — el pipeline de streaming.
- `src/app/state/screens.rs` — el stack de pantallas.
- `src/app/app/events.rs` — cómo los eventos de background llegan a
  la UI.
- `src/plugin/runtime/runtime.rs` — el runtime de plugins Lua.
