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

## Ejemplos

### `peek()` sobre un archivo `.zip`

Pasa el cursor por encima de `release-0.7.0.zip` en el panel activo.
El panel de previsualización se reemplazará por un widget
`pairee.ui.Table` como el siguiente:

```text
┌──────────────────────────────┬───────┬─────────────────────┐
│ Ruta                         │ Tamaño│ Modificado          │
├──────────────────────────────┼───────┼─────────────────────┤
│ release-0.7.0/CHANGELOG.md   │ 1.2 K │ 2026-07-26 21:14:00 │
│ release-0.7.0/pairee         │ 24 M  │ 2026-07-26 21:14:00 │
│ release-0.7.0/pairee.sig     │  512  │ 2026-07-26 21:14:00 │
└──────────────────────────────┴───────┴─────────────────────┘
```

Usa `seek(job)` (o navega con el teclado en el panel de previsualización)
para paginar listas más largas.

### `entry()` sobre un archivo `.zip`

Pulsa `F2` con el cursor sobre `release-0.7.0.zip` para obtener un
resumen rápido como notificación:

```text
release-0.7.0.zip

3 entradas
Tamaño total: 24.0 M
```

## Resolución de problemas

| Síntoma | Causa probable | Solución |
|---------|----------------|----------|
| El panel de previsualización muestra "Could not list archive contents" | Falta la herramienta necesaria (`unzip`, `tar` o `7z`) en el `PATH` | Instálala: `apt install unzip / p7zip`, `brew install p7zip`, etc. |
| `peek()` devuelve una tabla vacía | El archivo está protegido con contraseña | El plugin no soporta archivos cifrados; descífralo primero o usa una herramienta que soporte la contraseña. |
| `F2` muestra "Not a supported archive: …" | El archivo bajo el cursor no es `.zip` / `.tar(.gz)` / `.tgz` / `.7z` | Es lo esperado; el plugin sólo inspecciona esas cuatro familias. |
| El listado parece truncado a unos cientos de entradas | El valor de `max_entries` es muy bajo | Súbelo desde `Opciones → Plugins → archive-inspect` (por defecto: `500`). |
| El orden de clasificación parece incorrecto | Se cambió `sort_by` del valor por defecto | Restáuralo a `path`, `size` o `date` desde el mismo diálogo. |
| El plugin no carga | El hash de `main.lua` en el [files] del `manifest.toml` ya no coincide con el archivo en disco | El lockfile fue alterado; reinstala con `pairee plugin install archive-inspect.pairee`. |
