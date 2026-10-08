# Guía de Atajos de Teclado

Esta guía recopila todas las combinaciones de teclas y atajos interactivos de Pairee, ordenados por categorías funcionales.

---

## 📂 1. Navegación de Paneles y Foco

| Tecla / Atajo | Acción |
| :--- | :--- |
| `Tab` | Cambia el foco de selección entre el panel izquierdo y derecho. |
| `Arriba` / `Abajo` | Desplaza el cursor de selección una posición. |
| `Re Pág` / `Av Pág` | Desplaza la lista de archivos una pantalla completa. |
| `Inicio` / `Fin` | Salta directamente al primer o último archivo de la lista. |
| `Ctrl+U` | Intercambia las rutas de directorio entre el panel izquierdo y derecho. |
| `Ctrl+H` | Alterna la visualización de archivos ocultos y de sistema. |
| `Ctrl+R` | Recarga y refresca el contenido del directorio del panel activo. |
| `Ctrl+\` | Abre la lista de favoritos de carpetas (`Enter` ir, `Ins`/`+` añadir la carpeta actual, `Supr`/`-` quitar). Se guarda en `bookmarks.toml`. |
| `Ctrl+Alt+1` … `Ctrl+Alt+9` | Salta al atajo de carpeta 1–9. Se asignan desde **Comandos → Accesos dir. carp.** (`Ins`/`Espacio` o el dígito del atajo asigna la carpeta actual, `Supr` lo borra). |

---

## 🗂️ 2. Modos de Vista y Visibilidad

| Tecla / Atajo | Acción |
| :--- | :--- |
| `Ctrl+1` | **Brief (Breve):** Nombres de archivo únicamente en varias columnas. |
| `Ctrl+2` | **Medium (Medio):** Nombre y extensión lado a lado. |
| `Ctrl+3` | **Full (Detallado):** Vista estándar (Nombre, Tamaño, Fecha). |
| `Ctrl+4` | **Wide (Ancho):** Listado con nombres ensanchados. |
| `Ctrl+5` | **Detailed (Completo):** Permisos Unix, propietarios, enlaces duros. |
| `Ctrl+6` | **Descriptions (Descripciones):** Renderiza descripciones de `Descript.ion`. |
| `Ctrl+7` | **File owners (Propietarios):** Muestra usuarios y grupos del sistema. |
| `Ctrl+8` | **File links (Enlaces):** Muestra el contador de enlaces duros. |
| `Ctrl+9` | **Alt Full:** Vista configurable personalizada por el usuario. |
| `Ctrl+F1` | Muestra / oculta el panel izquierdo. |
| `Ctrl+F2` | Muestra / oculta el panel derecho. |
| `Ctrl+O` | Muestra / oculta ambos paneles (inspecciona la salida de consola). |
| `Ctrl+Q` | Alterna la **Vista Rápida** de archivos en el panel opuesto. |
| `Ctrl+L` | Alterna el **Panel de Información del Sistema** en el panel activo. |

---

## 🛠️ 3. Acciones de Archivo Estándar (Teclas F)

| Tecla | Acción |
| :--- | :--- |
| `F1` | Abre el visualizador de ayuda y manuales del sistema. |
| `F2` | Abre el menú de comandos definidos por el usuario. |
| `F3` | Abre el visor de archivos interno (modos Texto o Hexadecimal). Sobre una carpeta: calcula su tamaño. |
| `Alt+F3` | Abre el visor de archivos interno en modo alternativo. |
| `F4` | Abre el editor integrado (ver [sección 7](#-7-editor-integrado-f4)). |
| `F5` | Copia los archivos seleccionados hacia el panel opuesto. |
| `Alt+F5` | Imprime archivos aplicando un comando filtro. |
| `F6` | Mueve los elementos seleccionados al panel opuesto. Pulsa `Tab` desde el campo de ruta para ver las opciones avanzadas (enlaces simbólicos, atributos, filtro, etc.). |
| `Alt+F6` | Abre el diálogo de creación de enlaces simbólicos o duros. |
| `F7` | Renombra el archivo resaltado en el mismo directorio (solo nombre). |
| `Shift+F6` | Renombrado múltiple de los elementos seleccionados con máscaras, contador, buscar y reemplazar y vista previa en vivo (ver Funciones, sección 2.2.1). En el diálogo: `Tab`/flechas cambian de campo, `Espacio` marca opciones, `Izquierda`/`Derecha` cambian el modo de mayúsculas, `RePág`/`AvPág` desplazan la vista previa, `Enter` renombra, `Esc` cancela. |
| `F8` / `Delete` | Elimina los archivos seleccionados o marcados. |
| `Alt+Delete` | Borrado Seguro (Wipe): Sobrescribe sectores de datos antes de borrar. |
| `F9` | Activa la barra superior de menú desplegable. |
| `F10` | Cierra y sale de la aplicación. |
| `F12` | Abre el listado de pantallas y pestañas activas de fondo. |
| `Esc` | Limpia comandos de consola, cierra popups o sale de menús. |
| `Menu` / `Alt+M` | Abre el menú contextual del archivo seleccionado. |

> **Consejo:** Las acciones que antes estaban en `F7` (Crear carpeta) y `F11` (Plugins) ahora se encuentran en **Barra de menú** → submenú **Archivos**. La opción **Crear carpeta** también está disponible por defecto (tecla `6`) en el **Menú de Usuario** (`F2`).

---

## 🏷️ 4. Marcado y Selección Múltiple

| Tecla | Acción |
| :--- | :--- |
| `Insert` / `Espacio` | Marca/desmarca archivos. El cursor avanza hacia abajo. Sobre una carpeta también calcula su tamaño (`Esc` lo detiene). |
| `+` (Teclado numérico) | Marca un grupo de archivos según un patrón wildcard (ej. `*.rs`). |
| `-` (Teclado numérico) | Desmarca un grupo de archivos según patrón wildcard. |
| `*` (Teclado numérico) | Invierte la selección en todo el panel. |
| `Ctrl+M` | Restaura el último grupo de selección marcado. |
| `Ctrl+I` | Aplica un filtro de búsqueda persistente en el panel activo. |

> **TOML de atajos:** los acordes Far `Gray+` / `Gray-` / `Gray*` se aceptan y se traducen a `Plus` / `-` / `*`. En archivos nuevos usa `Plus`. Los acordes inválidos o duplicados se rechazan; Ajustes → Interfaz los lista.

> **Limitación de la terminal (`Ctrl+H`, `Ctrl+I`, `Ctrl+M`):** las terminales clásicas envían los mismos bytes para `Ctrl+H` y `Retroceso`, `Ctrl+I` y `Tab`, `Ctrl+M` y `Enter`. Estos tres atajos solo funcionan en terminales compatibles con el protocolo de teclado kitty (kitty, WezTerm, foot, Ghostty, Alacritty, Windows Terminal reciente, …). En las demás, reasígnalos en `keybindings.toml`:
>
> ```toml
> [custom_bindings]
> toggle_hidden     = "Alt+h"
> file_panel_filter = "Alt+i"
> restore_selection = "Alt+r"
> ```

---

## 🌐 5. SSH, Git y Herramientas de Sistema

| Tecla / Atajo | Acción |
| :--- | :--- |
| `Ctrl+Shift+S` | Abre el cuadro de diálogo de conexión SSH / SFTP. |
| `Alt+G` | Abre el panel de integración con Git. |
| `Ctrl+W` | Abre el administrador de procesos activos del sistema operativo. |
| `Alt+F10` | Abre el navegador de directorios en árbol gráfico. |
| `Ctrl+p` / `Ctrl+P` | Cambia manualmente la barra de teclas F (Normal -> Ctrl -> Alt). |
| `s` | (Con flujo Yazi activo) Abre el menú inferior modal de ordenación. |
| `v` | (Con flujo Yazi activo) Abre el menú inferior modal de vista. |
| `Ctrl+Tab` | Salta a la siguiente pestaña abierta de fondo. |
| `Ctrl+Shift+Tab` | Regresa a la pestaña anterior de fondo. |

---

## 🔗 6. Editor de Asociaciones de Archivos

| Tecla / Atajo | Acción |
| :--- | :--- |
| `↑` / `↓` | Navegar por la lista de reglas. |
| `A` / `a` / `Insert` | Añadir una nueva regla de asociación. |
| `E` / `e` / `Enter` | Editar la regla de asociación seleccionada. |
| `D` / `d` / `Delete` | Eliminar la regla de asociación seleccionada. |
| `Esc` | Cerrar el editor o cancelar el paso de edición actual. |

---

## 📝 7. Editor integrado (F4)

Pairee edita los archivos siempre con su editor integrado; nunca lanza un editor externo. Se abre con `F4` en un panel, con `F6` en el visor y desde **Comandos → Editar menú de usuario**.

| Tecla / Atajo | Acción |
| :--- | :--- |
| `F2` / `Ctrl+S` | Guardar. Si otro programa modificó el archivo desde que se abrió, se pide confirmación antes de sobrescribirlo. |
| `Shift+F2` | Guardar como (un nombre relativo se resuelve respecto de la carpeta del archivo; un archivo existente solo se sobrescribe tras confirmar). |
| `Ctrl+Z` | Deshacer (una racha de escritura se deshace en un solo paso). |
| `Ctrl+Y` / `Ctrl+Shift+Z` | Rehacer. |
| `F7` / `Ctrl+F` | Buscar. `F3` / `Shift+F7` repiten la última búsqueda. |
| `Ctrl+R` | Recargar el archivo desde disco (pregunta antes si hay cambios sin guardar y la confirmación está activa). |
| `Inicio` / `Fin` | Inicio / fin de línea. `Ctrl+Inicio` / `Ctrl+Fin` van al inicio / final del archivo. |
| `Tab` | Inserta una tabulación, o espacios hasta la siguiente parada si **Expandir tabulaciones** está activo. |
| `F4` | Abre el archivo en el visor (modo hexadecimal). |
| `F8` | Descartar cambios y cerrar. |
| `Esc` / `F10` | Cerrar (pregunta si hay cambios sin guardar). |

El editor conserva los finales de línea del archivo (LF o CRLF), el salto de línea final y el BOM UTF-8. Los archivos que no son UTF-8 válido o que superan 64 MiB no se abren (usa el visor).
