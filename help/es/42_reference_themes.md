# 📖 Referencia: Esquema TOML de Temas

> **Cuadrante: REFERENCE** — *orientado a información.*

Pairee carga temas desde:

- **Windows:** `%APPDATA%\pairee\config\themes\`
- **Linux / macOS:** `~/.config/pairee/themes/`

Cada archivo es un documento TOML. El tema activo se selecciona en
`Configuration → Colors → Theme` (el mismo diálogo también tiene un
toggle para los dos temas bundled, `slate` y `classic_blue`).

---

## Ejemplo mínimo

```toml
[panel]
border          = "Blue"
background      = "Black"
file_selected   = "Yellow"
file_directory  = "Cyan"
file_executable = "Green"

[menu]
background = "Blue"
selected   = "White"
```

Eso alcanza para tener un look distinto. Pairee cae a un default para
cualquier clave que no seteés.

---

## Grupos de nivel superior

| Grupo | Propósito |
| --- | --- |
| `[panel]` | Borde del panel, fondo, colores de tipo de archivo. |
| `[menu]` | Colores de la barra de menú superior. |
| `[dialog]` | Popups de diálogo (modales, selectores de lista, prompts). |
| `[viewer]` | Colores del visor interno de archivos. |
| `[editor]` | Colores del editor interno. |
| `[status]` | Línea de status al fondo del panel. |
| `[clock]` | Widget de reloj arriba a la derecha. |
| `[fkey]` | Barra F-key (números, texto). |
| `[transfer]` | Popup de progreso de transferencia. |
| `[highlight]` | Reglas de resaltado por extensión. |

La mayoría de los grupos siguen las mismas claves de color. Algunos
grupos tienen claves específicas de comportamiento (ej. `[fkey]` tiene
`fkey_bg`, `fkey_num_fg`, `fkey_text_fg`).

---

## Valores de color

Un color puede ser cualquiera de:

| Forma | Ejemplos |
| --- | --- |
| **Color con nombre** | `Black`, `Red`, `Green`, `Yellow`, `Blue`, `Magenta`, `Cyan`, `White`, `Gray`, `DarkGray`, `Reset` |
| **Hexadecimal (6 dígitos)** | `"#FF7700"`, `"#1B2A3F"` |
| **Hexadecimal (triplete RGB)** | `"(255, 119, 0)"` |

Se aceptan tanto mayúsculas como minúsculas en los hex.

> Algunas terminales (Windows Console, viejo `cmd.exe`) solo renderean
> 16 colores con nombre. Si especificás un valor hex y el color no
> aparece, probá un color con nombre o cambiá a Windows Terminal /
> WezTerm.

---

## Claves de `[panel]`

| Clave | Default | Significado |
| --- | --- | --- |
| `border` | `"Blue"` | Borde del panel activo. |
| `border_inactive` | `"DarkGray"` | Borde del panel pasivo. |
| `background` | `"Black"` | Fondo interno del panel. |
| `selected` | `"Yellow"` | Color de tag para la fila resaltada. |
| `file_selected` | `"Yellow"` | Color de tag sobre las filas seleccionadas. |
| `file_directory` | `"Cyan"` | Carpetas. |
| `file_executable` | `"Green"` | Binarios y scripts. |
| `file_symlink` | `"Magenta"` | Symbolic links. |
| `file_archive` | `"Red"` | Archivos (zip, tar, 7z, …). |
| `file_image` | `"Yellow"` | Archivos de imagen. |
| `file_temp` | `"Gray"` | Archivos temporales / cache. |
| `file_hidden` | `"DarkGray"` | Archivos ocultos / del sistema. |

---

## Claves de `[menu]`

| Clave | Default | Significado |
| --- | --- | --- |
| `background` | `"Blue"` | Fondo de la barra de menú superior. |
| `selected` | `"White"` | Texto del item resaltado. |
| `unselected` | `"Black"` | Texto de items no resaltados. |
| `shortcut` | `"Yellow"` | Letra mnemónica (acelerador `&`). |
| `separator` | `"DarkGray"` | Líneas separadoras. |

---

## Claves de `[dialog]`

| Clave | Default | Significado |
| --- | --- | --- |
| `border` | `"Blue"` | Borde. |
| `background` | `"Black"` | Fondo del cuerpo. |
| `title` | `"Yellow"` | Texto del título. |
| `text` | `"White"` | Texto del cuerpo. |
| `button` | `"Cyan"` | Texto del botón. |
| `button_active` | `"Yellow"` | Botón con foco. |
| `input` | `"White"` | Texto del input. |
| `input_active` | `"Yellow"` | Input con foco. |

---

## Claves de `[viewer]` / `[editor]`

| Clave | Default | Significado |
| --- | --- | --- |
| `border` | `"Blue"` | Borde. |
| `background` | `"Black"` | Fondo del cuerpo. |
| `text` | `"White"` | Texto plano. |
| `selection` | `"Yellow"` | Texto seleccionado. |
| `cursor` | `"White"` | Caret. |
| `line_number` | `"DarkGray"` | Gutter de número de línea. |
| `search_hit` | `"Yellow"` | Resaltado de match de búsqueda. |
| `hex_byte` | `"Cyan"` | Dígitos hex. |
| `hex_ascii` | `"Gray"` | Lado ASCII. |

---

## Claves de `[status]` / `[clock]` / `[fkey]`

| Grupo | Clave | Default | Significado |
| --- | --- | --- | --- |
| `status` | `background` | `"Black"` | Fondo de la línea de status. |
| `status` | `text` | `"Gray"` | Texto de la línea de status. |
| `status` | `selected_count` | `"Yellow"` | Conteo de selección resaltado. |
| `clock` | `background` | `"Black"` | Fondo del reloj. |
| `clock` | `text` | `"White"` | Dígitos del reloj. |
| `fkey` | `fkey_bg` | `"DarkGray"` | Fondo de la barra F-key. |
| `fkey` | `fkey_num_fg` | `"Black"` | Foreground del número F-key. |
| `fkey` | `fkey_text_fg` | `"White"` | Foreground del label F-key. |

---

## Claves de `[transfer]`

| Clave | Default | Significado |
| --- | --- | --- |
| `bar` | `"Cyan"` | Relleno de la barra de progreso. |
| `bar_bg` | `"DarkGray"` | Fondo de la barra de progreso. |
| `text` | `"White"` | Nombre de archivo y stats. |
| `speed` | `"Yellow"` | Velocidad / ETA. |
| `error` | `"Red"` | Texto de error. |

---

## `[highlight]` — reglas por extensión

La tabla `[highlight]` te deja mapear máscaras de archivo a un color:

```toml
[highlight]
"*.rs"   = "Yellow"
"*.toml" = "Cyan"
"*.md"   = "Green"
"*.lock" = "DarkGray"
```

Una máscara sigue reglas de glob (`*`, `?`, `[abc]`). El color es el
mismo formato que en cualquier otro lado (con nombre o hex).

---

## Usar un tema custom

1. Guardá tu archivo TOML en el directorio `themes/`.
2. Abrí `F9` → `Options` → `Configuration` → **Colors**.
3. La primera fila alterna el tema bundled. Bajá para encontrar tu
   tema custom en el picker.
4. `Enter` para aplicar; `F9` de nuevo para guardar.

> Los cambios se escriben en `settings.toml` inmediatamente si
> `auto_save_setup = true` (el default).

---

## Troubleshooting

| Síntoma | Fix probable |
| --- | --- |
| Mi tema no aparece en el picker | Verificá que el archivo sea `.toml` y viva en el directorio `themes/` correcto. |
| Los colores se ven mal (lavados) | Tu terminal puede no soportar true color. Probá Windows Terminal, WezTerm, Alacritty, o `COLORTERM=truecolor` en tu ambiente. |
| Un slot específico nunca cambia | El slot no está en el esquema de arriba; la clave se ignora. Usá el diálogo **Color groups** en `Configuration → Colors` para la lista completa de slots. |
| Los colores hex se renderean como el color con nombre más cercano | Tu terminal reporta `COLORTERM != truecolor`. Pairee cae con gracia pero perdés la precisión. |

---

## A dónde ir ahora

- Referencia de configuración: [`41_reference_configuration`](41_reference_configuration.md)
- Cómo cambiar de tema: [`25_howto_appearance_themes`](25_howto_appearance_themes.md)
