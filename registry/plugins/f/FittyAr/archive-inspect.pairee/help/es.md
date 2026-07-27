# archive-inspect.pairee

Un plugin de previsualización para Pairee que lista el contenido de archivos
ZIP, TAR.GZ y 7Z directamente en el panel de previsualización, sin
necesidad de extraerlos.

## ¿Cuándo se activa?

El plugin se registra automáticamente como previsualizador para archivos
con las siguientes extensiones:

- `.zip`
- `.tar`, `.tar.gz`, `.tgz`
- `.7z`

Al pasar el cursor sobre un archivo comprimido, el panel de previsualización
mostrará una tabla con su contenido (ruta, tamaño, fecha de modificación).

## Atajos de teclado

| Tecla | Acción |
|-------|--------|
| `F2`  | Muestra un resumen emergente del archivo bajo el cursor |

## Configuración

Puedes ajustar el plugin desde el diálogo **Opciones** de Pairee
(pestaña Plugins):

| Opción | Por defecto | Descripción |
|--------|-------------|-------------|
| `max_entries` | `500` | Número máximo de entradas a mostrar. Listas más largas se truncan. |
| `show_hidden` | `false` | Incluir archivos ocultos (dotfiles). |
| `sort_by` | `path` | Columna de orden por defecto: `path`, `size` o `date`. |
| `extra_args` | `""` | Argumentos extra añadidos a la herramienta de listado. |

## Herramientas necesarias

El plugin invoca los siguientes binarios (deben estar en el `PATH`):

- `unzip` para archivos `.zip`
- `tar` para archivos `.tar`/`.tar.gz`/`.tgz`
- `7z` para archivos `.7z` (CLI de 7-Zip, p. ej. `p7zip` en Linux)

Por este motivo, el plugin se ejecuta en modo **confiable** (trusted):
se te pedirá que confíes en él la primera vez que lo instales.

## Cómo funciona

El plugin implementa el contrato de previsualizador:

- `peek(job)` — detecta el tipo de archivo, lanza la herramienta de listado
  adecuada, parsea la salida, ordena/limita según la configuración del
  usuario y devuelve un widget `pairee.ui.Table`.
- `seek(job)` — reemite las entradas en caché cuando el usuario hace scroll.
- `entry()` — se invoca con `F2`; muestra un popup con el número de
  entradas y el tamaño total descomprimido.

La salida de cada herramienta se parsea con un parser pequeño y propio,
así que el plugin no tiene dependencias externas en Lua.
