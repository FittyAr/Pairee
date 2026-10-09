# Si vienes de otro programa

Cada preset conserva las teclas de una familia de programas. Esta página dice
qué encontrarás como lo dejaste, qué agrega Pairee y qué cambia. Las listas
completas están en las páginas **Teclas:**.

---

## Norton Commander, Far Manager, Midnight Commander → preset *Norton Commander / Far*

**Como lo conoces.** La fila de teclas F (`F1` ayuda, `F2` menú de usuario,
`F3` ver, `F4` editar, `F5` copiar, `F6` mover, `F7` crear carpeta, `F8`
borrar, `F9` menú, `F10` salir, `F11` plugins, `F12` pantallas), `Shift+F4`
archivo nuevo, `Shift+F6` renombrar, `Alt+F1`/`Alt+F2` unidades, `Alt+F7`
buscar, `Alt+F8` historial de comandos, `Alt+F10` árbol, `Alt+F11`/`Alt+F12`
historial de archivos vistos y de carpetas, `Ctrl+F1`/`Ctrl+F2` paneles,
`Ctrl+O`, `Ctrl+U`, `Ctrl+L`, `Ctrl+Q`, `Ctrl+P` oculta el panel pasivo,
`Ctrl+B` la barra de teclas, `Ctrl+H` archivos ocultos, `Ctrl+R` releer,
`Ctrl+A` atributos, `Ctrl+G` aplicar comando, `Ctrl+Z` describir, `Ctrl+W`
tareas, `Ctrl+F3`…`Ctrl+F12` ordenar, `Ctrl+1`…`Ctrl+9` vistas, `Insert` y
`+` `-` `*` del teclado numérico marcan, `Ctrl+M` restaura la selección,
`Ctrl+\` va a la raíz, `Ctrl+PgUp`/`Ctrl+PgDn`. Las letras van a la línea de
comandos; `Ctrl+Enter`, `Ctrl+F`, `Ctrl+[` y `Ctrl+]` insertan nombres y rutas,
`Ctrl+E`/`Ctrl+X` recorren el historial y `Ctrl+Y` la borra. `Alt` + una letra
es la búsqueda rápida.

**Lo que agrega Pairee** está en `Ctrl+Alt` + una letra, para que `Alt` + letra
quede libre para la búsqueda rápida: `T` / `W` nueva / cerrar pestaña, `O`
abrir en pestaña nueva, `G` Git, `R` SSH, `S` tamaño de carpetas, `D` uso de
disco, `B` favoritos, `F` filtro rápido. `Alt+1`…`Alt+9` y
`Alt+PgUp`/`Alt+PgDn` cambian de pestaña, `Alt+Retroceso` deshace una
operación de archivos (`Alt+Shift+Retroceso` la rehace), `Ctrl+Insert` copia
nombres, `Alt+Shift+Insert` rutas, y `Ctrl+K` lista todas las teclas.

**Cambia.** `Ctrl+Shift+F6` es el renombrado múltiple y `Ctrl+T` el panel de
transferencias (el panel de árbol de Far es `Alt+F10`). `Shift+Supr` borra sin
pasar por la papelera. En Windows, `Ctrl+Alt` es `AltGr` y con algunas
distribuciones escribe un carácter; todas estas acciones están también en el
menú (`F9`).

---

## Explorador de Windows, VS Code, Total Commander → preset *Estándar*

**Como lo conoces.** `Ctrl+C` / `Ctrl+X` / `Ctrl+V` copian, cortan y pegan
archivos, `Supr` manda a la papelera y `Shift+Supr` borra, `F2` renombra,
`Ctrl+Z` / `Ctrl+Y` deshacen y rehacen, `Ctrl+A` marca todo, `Ctrl+F` busca en
la carpeta, `Alt+←` / `Alt+→` atrás y adelante, `Alt+↑` y `Retroceso` suben,
`Ctrl+T` / `Ctrl+W` / `Ctrl+Tab` manejan pestañas, `Ctrl+Shift+N` crea una
carpeta, `Ctrl+N` un archivo, `Alt+Enter` muestra propiedades, `Ctrl+Shift+P`
abre la paleta de comandos, `Ctrl+,` los ajustes, `` Ctrl+` `` la línea de
comandos, `Ctrl+K Ctrl+S` la lista de atajos, `Ctrl+Q` sale. Escribir un
nombre salta al archivo.

**De Total Commander,** porque Pairee tiene dos paneles: `F3` ve, `F4` edita,
`F5` copia y `F6` mueve al otro panel, `F7` crea una carpeta, `F8` borra,
`Ctrl+M` es el renombrado múltiple y `Ctrl+U` intercambia los paneles.

**Cambia.** `F5` copia en vez de actualizar: las carpetas se actualizan solas
y `Ctrl+R` relee. Buscar siguiente es `Ctrl+G`, porque `F3` ve. `F10` abre la
barra de menú. Las pestañas usan `Ctrl+Tab`, así que las pantallas (editor,
visor) usan `F12` y `Ctrl+F12`. `Ctrl+Shift` + una letra necesita un terminal
que la informe; cada una de esas acciones tiene otra tecla (`Ctrl+Alt+F`
busca archivos, `F7` crea una carpeta, `Alt+C` copia la ruta).

---

## Vim, Neovim, vifm, oil.nvim, nvim-tree → preset *Neovim*

**Como lo conoces.** `h` `j` `k` `l`, `gg` / `G`, `Ctrl+U` / `Ctrl+D`,
`Ctrl+B` / `Ctrl+F`, `/` `n` `N`, `v` / `V` selección visual, `u` / `Ctrl+R`
deshacer y rehacer, `gt` / `gT` pestañas, `Ctrl+W` `w`/`h`/`l`/`x`/`o` se
mueve entre los paneles, los intercambia y los oculta como ventanas,
`Ctrl+O` / `Ctrl+I` atrás y adelante, `K` muestra detalles, `ZZ` sale, `g?`
lista las teclas. El líder es `Espacio`, como en LazyVim: `Espacio f f` busca
archivos, `Espacio f r` carpetas recientes, `Espacio g g` Git, `Espacio ?`
teclas.

**De los gestores de archivos:** `yy` copia al portapapeles, `p` pega una
copia y `P` pega sobrescribiendo, `x` corta, `dd` manda a la papelera y `DD`
borra, `cw` renombra (`cW` solo el nombre), `a` crea un archivo (una carpeta
si el nombre termina en `/`), `-` sube (vim-vinegar, oil.nvim), `g.` o `za`
muestra los ocultos, `t` marca (vifm), `gx` abre el menú contextual (abrir con...), `yp` / `yn` copian la
ruta / el nombre.

**Cambia.** `:` abre la paleta de comandos (buscar una acción por nombre) y
`!` la línea de comandos del shell. No hay conteos (`5j`). El editor integrado
no es modal: conserva las teclas de edición habituales y `Esc` lo cierra.
`Ctrl+I` llega como `Tab` en terminales antiguos. Las teclas F de Norton
siguen funcionando (`F5` copiar, `F7` carpeta...).

---

## yazi → preset *yazi*

**Como lo conoces.** `h` `j` `k` `l`, `H` / `L` atrás y adelante, `gg` / `G`,
`Ctrl+U` / `Ctrl+D` / `Ctrl+B` / `Ctrl+F`, `Espacio` marca y baja, `v` / `V`
visual, `Ctrl+A` todo, `Ctrl+R` invierte, `y` / `x` / `p` / `P` copian, cortan,
pegan y pegan sobrescribiendo, `Y` / `X` vacían el portapapeles, `-` enlaza,
`d` / `D` papelera y borrar, `a` crea (con `/` al final, una carpeta), `r`
renombra, `.` archivos ocultos, `f` filtra, `/` `n` `N` buscan, `s` / `S`
buscan archivos y contenidos, `z` busca un archivo, `Z` salta a una carpeta
visitada, `cc` / `cd` / `cf` / `cn` copian ruta, carpeta, nombre y nombre sin
extensión, `,` + una letra ordena, `m` + una letra cambia las columnas, `t`
nueva pestaña, `1`…`9` cambian de pestaña, `[` / `]` anterior / siguiente,
`{` / `}` la mueven, `Ctrl+C` la cierra, `w` tareas, `;` / `:` shell, `~`
teclas, `q` / `Q` salir, `o` / `l` / `Enter` abrir, `O` el menú contextual (abrir con...). Las
secuencias esperan su siguiente tecla.

**Cambia.** Pairee muestra dos paneles: `Tab` pasa de uno a otro e `i` muestra
el panel de información (el "spot" de yazi); `Ctrl+Q` muestra una vista
previa del archivo en el otro panel. Ordenar al revés es `, r` en vez de las
letras mayúsculas. Hay un solo tipo de enlace (`_` no se usa). `e` edita con
el editor integrado. `z` y `Z` usan la búsqueda y el historial de carpetas de
Pairee en lugar de fzf y zoxide. Las teclas F de Norton siguen funcionando
(`F5` copiar, `F7` carpeta...).
