# Atajos de teclado

Pairee no te pide aprender un teclado nuevo. Elige el preset del programa que
tus dedos ya conocen y cada tecla funciona como funcionaba allí. Esta página
explica cómo funcionan las teclas; la lista completa de cada preset está en
las páginas **Teclas:** de esta ayuda y, dentro de Pairee, en la lista de
atajos.

---

## 1. Presets

| Preset | Para quien viene de | En resumen |
| :--- | :--- | :--- |
| **Norton Commander / Far** | Norton Commander, Far Manager, Midnight Commander | `F3` ver, `F5` copiar, `F6` mover, `F7` carpeta, `F8` borrar, lo que escribes va a la línea de comandos, `Alt`+letra búsqueda rápida |
| **Estándar** | Explorador de Windows, VS Code, Total Commander | `Ctrl+C/X/V` con archivos, `Supr` a la papelera, `F2` renombrar, `Ctrl+T` pestañas, escribir salta a un archivo |
| **Neovim** | Vim, Neovim, vifm, oil.nvim, nvim-tree | `hjkl`, `gg`/`G`, `yy`/`dd`/`p`, `/` `n` `N`, líder `Espacio` |
| **yazi** | yazi | `hjkl`, `y`/`x`/`p`, `d`/`D`, `a` crear, `.` ocultos, `~` atajos |

Elige uno la primera vez que abres Pairee, o después en **Opciones →
Configuración → Interfaz → Preajuste de atajos de teclado**, o en la lista de atajos (`Ctrl+←`/`Ctrl+→`
y luego `F5`). Los presets Neovim y yazi conservan todas las teclas F de
Norton Commander, así que `F5` sigue copiando.

La página **Si vienes de otro programa** cuenta qué es igual y qué cambia en
cada caso.

---

## 2. La lista de atajos

Ábrela con la tecla de tu preset (`Ctrl+K` en Norton, `Ctrl+K Ctrl+S` en
Estándar, `g?` en Neovim, `~` en yazi; `Ctrl+Shift+K` en todos si el terminal
la envía), desde el menú (`F9`) **Opciones → Atajos de teclado...** o desde
**Opciones → Configuración → Interfaz → Atajos de teclado…**.

Muestra cada comando de los paneles, el editor, el visor y las listas de los
diálogos (`Tab` cambia entre ellos), agrupados por categoría, incluidos los
que no tienen tecla y los de tus plugins. Marcas delante de cada fila:

| Marca | Significado |
| :--- | :--- |
| `●` | Cambiaste las teclas de este comando. |
| `⚠` | Un plugin sugirió una tecla que ya estaba en uso, así que no tiene ninguna. |
| `≈` | Sus teclas necesitan un terminal capaz (ver la sección 5). |
| `∅` | Sin tecla: usa la paleta de comandos o asígnale una. |

| Tecla | En la lista |
| :--- | :--- |
| *escribir* | Filtra por nombre, id o tecla. |
| `F3` | Pulsa cualquier tecla para ver qué hace. |
| `Enter` | Ejecuta la acción marcada. |
| `F2` | Da una tecla nueva a la acción: púlsala (o una secuencia) y `Enter`. Si otra acción la usa, Pairee pregunta antes. |
| `Ins` | Añade una tecla más a la acción. |
| `Supr` | Quita las teclas de la acción. |
| `F8` / `Shift+F8` | Devuelve a la acción (o a todas) las teclas del preset. |
| `Ctrl+←` / `Ctrl+→` | Muestra otro preset; `F5` lo vuelve el activo. |
| `F9` | Guarda el preset con tus cambios como un archivo de preset propio. |

Los cambios se guardan al instante y funcionan enseguida. La **paleta de
comandos** (`Ctrl+Shift+P`, `Ctrl+Alt+P`, `:` en Neovim) lista cada acción con
su tecla.

---

## 3. Cómo funcionan las teclas

- **Chords**: una tecla con modificadores: `F5`, `Ctrl+c`, `Alt+Shift+PageUp`.
  Una letra mayúscula significa `Shift` (`Ctrl+K` es `Ctrl+Shift+k`).
- **Secuencias**: varias teclas una tras otra: `g g`, `Ctrl+w h`. Mientras hay
  una en curso, abajo se ven las teclas que pueden seguir; `Esc` la cancela.
  Una secuencia espera un segundo su siguiente tecla (en yazi espera lo que
  haga falta).
- **La tecla líder** (Neovim: `Espacio`) inicia casi todas las secuencias de
  Neovim: `Espacio f f` busca archivos.
- **Las letras** hacen lo que hacían en tu programa: en Norton escriben en la
  línea de comandos, en Estándar saltan al archivo cuyo nombre empieza así, y
  en Neovim y yazi son comandos (una letra sin comando no hace nada; `!`, `;`
  o `:` abren la línea de comandos).
- **Búsqueda rápida** (Norton): `Alt` y una letra busca por nombre; sigue
  escribiendo, `↑`/`↓` pasan de una coincidencia a otra, `Esc` cierra.
- **Contextos**: los paneles, el editor, el visor y las listas de los diálogos
  tienen sus propias teclas. La ayuda, la lista de pantallas, la paleta de
  comandos y la lista de atajos funcionan en todas partes.

---

## 4. Cambiar teclas en archivos

Todo lo que hace la lista de atajos se escribe en `keybindings.toml`, en la
carpeta de configuración, que también puedes editar:

```toml
preset = "norton"

[overrides.all]            # todos los presets
copy = "F5, Ctrl+Alt+c"    # varias teclas: separadas por comas
quick_view = ""            # sin tecla

[overrides.norton]         # solo este preset
"editor.save" = "Ctrl+s"   # las teclas de editor, visor y listas llevan prefijo
"viewer.quit" = "q, Esc"
"plugin.git-blame.toggle" = "Ctrl+Alt+g"
```

Nombrar una acción reemplaza sus teclas, y una tecla que tenía otra acción
pasa a ella. Un preset propio va en la carpeta `keymaps` (`F9` en la lista
escribe uno) y solo lista lo que cambia:

```toml
extends = "neovim"

[options]
typing = "commands"        # cli | type_ahead | commands
leader = "Space"
sequence_timeout = 1000    # milisegundos; 0 espera lo necesario
alt_quick_search = false

[panels]
go_parent = "h, -"

[editor]
save = "Ctrl+s, F2"
```

Los problemas de estos archivos (una acción desconocida, una tecla que no se
puede escribir, dos acciones en una tecla) aparecen en **Opciones →
Configuración → Interfaz → Ver problemas del mapa de teclas…**.

---

## 5. Límites de los terminales

Algunas teclas solo llegan si el terminal las informa; la lista las marca
con `≈`:

- `Ctrl+Shift+letra`, `Ctrl+Enter`, `Ctrl+Tab` y `Ctrl+dígito` necesitan un
  terminal con el protocolo de teclado de kitty (kitty, WezTerm, foot,
  Ghostty, Windows Terminal).
- `Ctrl+i`, `Ctrl+m`, `Ctrl+h` y `Ctrl+[` son lo mismo que `Tab`, `Enter`,
  `Retroceso` y `Esc` en terminales antiguos.
- En Windows `Ctrl+Alt` es `AltGr`: con algunas distribuciones de teclado
  `Ctrl+Alt+2` o `Ctrl+Alt+E` escriben un carácter.
- Algunos terminales se quedan con `F11`, `Alt+F4` o `Alt`+flechas.

Cada acción esencial tiene al menos una tecla que funciona en todas partes, y
todas las acciones están también en la barra de menú (`F9`/`F10`) y en la
paleta de comandos.

---

## 6. Teclas de los plugins

Los plugins sugieren teclas para sus comandos. Una tecla que tu preset ya usa
nunca se les da: el comando queda sin tecla (marcado `⚠` en la lista) y
puedes asignarle una. Ver **Plugins** para escribir plugins con teclas.

---

## 7. Editor y visor

Pairee edita los archivos siempre con su editor integrado; nunca lanza un
editor externo. Sus comandos (guardar, buscar, copiar, deshacer, cerrar...)
y todas las teclas del visor vienen de tu preset y aparecen en las páginas
**Teclas:** y en la lista de atajos. Moverse y seleccionar texto funciona
igual en todos los presets:

| Tecla | En el editor |
| :--- | :--- |
| Flechas, `Inicio` / `Fin`, `RePág` / `AvPág` | Moverse; `Ctrl+Inicio` / `Ctrl+Fin` van al inicio / final del archivo. |
| `Shift` + esas teclas | Seleccionar texto. Arrastrar con el ratón también selecciona. |
| `Alt+Shift` + flechas / `Inicio` / `Fin` (también `Ctrl+Alt+Shift`) | Selección vertical por bloques (columnas), al estilo Far. `Alt` + arrastrar con el ratón también selecciona un bloque. |
| Escribir, `Retroceso`, `Supr` | Reemplazan / borran la selección. |
| `Tab` | Inserta una tabulación, o espacios hasta la siguiente parada si **Expandir tabulaciones** está activo. |

El modo bloque (`Ctrl+B` en todos los presets) hace que `Shift` + flechas y
arrastrar con el ratón seleccionen un bloque vertical, para terminales que se
quedan con `Alt+Shift` + flechas, como Windows Terminal.

El editor conserva los finales de línea del archivo (LF o CRLF), el salto de
línea final y el BOM UTF-8. Los archivos que no son UTF-8 válido o que
superan 64 MiB no se abren (usa el visor). Copiar y cortar usan el
portapapeles del sistema; si no está disponible (por SSH o en una sesión
Linux sin entorno gráfico) Pairee guarda el texto en su propio portapapeles,
así que copiar y pegar siguen funcionando entre pantallas de editor.

El visor lee los archivos por bloques, así que incluso los de varios
gigabytes se abren al instante. Su línea de estado muestra la codificación,
la línea actual (o la posición en hexadecimal) y el progreso del indexado o
de una búsqueda. La codificación se detecta sola (marca de orden de bytes,
UTF-16, UTF-8 y, si no, la página de códigos heredada más probable); el
comando de codificación del visor elige otra, y la detección se puede
desactivar en **Opciones → Configuración → Editor/Visor**.

---

## 8. Teclas dentro de los diálogos

Las listas de los diálogos (menús, historiales, selectores) se recorren con
las flechas, `RePág` / `AvPág` e `Inicio` / `Fin`; `Enter` elige y `Esc` cierra.
Los presets Neovim y yazi suman `j` / `k`, `gg` / `G` y `q`. Algunos diálogos
tienen teclas propias, indicadas en su línea inferior. El editor de
asociaciones de archivos:

| Tecla | Acción |
| :--- | :--- |
| `A` / `a` / `Insert` | Añadir una regla. |
| `E` / `e` / `Enter` | Editar la regla marcada. |
| `D` / `d` / `Supr` | Borrar la regla marcada. |
