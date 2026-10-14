# archive-inspect

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

## Comandos

| Comando | Teclas por defecto | Acción |
|---------|--------------------|--------|
| Archive summary | `Ctrl+Alt+e a` (Neovim `<leader>pa`, yazi `Alt+p a`) | Muestra un resumen emergente del archivo bajo el cursor |

El comando también está en la paleta de comandos. Puedes reasignarlo en
`keybindings.toml` con su id, `plugin.archive-inspect.summary`.

RePág / AvPág desplazan el listado en el panel de previsualización.

## Configuración

Abre el **Gestor de plugins**, selecciona el plugin y pulsa `c` (o Enter):

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

En **Modo Seguro** solo pueden ejecutarse los programas declarados en el
manifiesto, por eso `manifest.toml` los declara:

```toml
[permissions]
commands = ["unzip", "tar", "7z"]
```

## Cómo funciona

El plugin implementa el contrato de previsualizador:

- `peek(job)` — detecta el tipo de archivo, lanza la herramienta de listado
  adecuada, parsea la salida, ordena/limita según la configuración del
  usuario y devuelve un widget `pairee.ui.Table`.
- `entry(args)` — el comando `summary`; lee el archivo bajo el cursor de
  `pairee.cx` (la anotación `--- @sync entry` lo actualiza) y muestra el
  número de entradas y el tamaño total descomprimido.

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

Desplaza las listas largas con RePág / AvPág en el panel de previsualización.

### El comando `summary` sobre un archivo `.zip`

Pulsa `Ctrl+Alt+e a` con el cursor sobre `release-0.7.0.zip` para obtener un
resumen rápido como notificación:

```text
release-0.7.0.zip

3 entradas
Tamaño total: 24.0 M
```

## Resolución de problemas

| Síntoma | Causa probable | Solución |
|---------|----------------|----------|
| El panel de previsualización muestra "No se pudo listar el contenido del archivo" | Falta la herramienta necesaria (`unzip`, `tar` o `7z`) en el `PATH` | Instálala: `apt install unzip / p7zip`, `brew install p7zip`, etc. |
| `peek()` devuelve una tabla vacía | El archivo está protegido con contraseña | El plugin no soporta archivos cifrados; descífralo primero o usa una herramienta que soporte la contraseña. |
| El resumen muestra "No es un archivo compatible: …" | El archivo bajo el cursor no es `.zip` / `.tar(.gz)` / `.tgz` / `.7z` | Es lo esperado; el plugin sólo inspecciona esas cuatro familias. |
| El listado parece truncado a unos cientos de entradas | El valor de `max_entries` es muy bajo | Súbelo en la configuración de `archive-inspect` del Gestor de plugins (por defecto: `500`). |
| El orden de clasificación parece incorrecto | Se cambió `sort_by` del valor por defecto | Restáuralo a `path`, `size` o `date` desde el mismo diálogo. |
| El plugin no carga | Necesita Pairee 0.11.0 o posterior (API Lua 1.3.0) | Actualiza Pairee y reinstálalo con `pairee plugin install archive-inspect`. |
