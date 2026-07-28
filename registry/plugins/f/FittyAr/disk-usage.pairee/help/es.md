# disk-usage.pairee

Un plugin de comandos para Pairee que analiza el uso de disco del directorio
de trabajo del panel activo y muestra qué elementos son los más pesados.

## Atajo de teclado

| Tecla | Acción |
|-------|--------|
| `Ctrl+D` | Ejecuta el análisis de uso de disco sobre el cwd del panel activo |

El resultado se renderiza en el panel de previsualización como una tabla
ordenada (los más grandes primero) y una notificación reporta cuántos
elementos se escanearon.

## Configuración

| Opción | Por defecto | Descripción |
|--------|-------------|-------------|
| `depth` | `2` | Profundidad de recursión al medir cada entrada de nivel superior (`1` = sin recursión). |
| `top_n` | `20` | Cuántas de las entradas más grandes mostrar. |
| `include_hidden` | `false` | Incluir archivos ocultos / directorios con punto. |
| `extra_args` | `""` | Argumentos extra añadidos a la invocación de `du` (o PowerShell). |

## Cómo funciona

El plugin lanza un proceso externo para hacer el trabajo pesado:

- En **Linux / macOS** ejecuta `du -k --max-depth=<depth> <cwd>`.
- En **Windows** ejecuta un pequeño pipeline de PowerShell que recorre el
  directorio con `Get-ChildItem -Recurse` y reporta los totales en bytes.

La salida se parsea, se ordena por tamaño, se recorta a las N entradas más
grandes y se formatea como un widget `pairee.ui.Table` que se empuja al
panel de previsualización.

## ¿Por qué es confiable?

El plugin necesita lanzar `du` / `powershell`, por lo que se ejecuta en
modo **confiable** (trusted). Se te pedirá que confíes en él la primera
vez que lo instales.

## Ejemplos

### Ejecución por defecto (depth=2, top_n=20)

Con el cursor parado sobre una carpeta de proyecto típica, pulsa
`Ctrl+D`. El panel de previsualización se reemplazará por un widget
`pairee.ui.Table` similar a:

```text
┌──────────────────┬──────────────────────────┐
│ Tamaño           │ Ruta                     │
├──────────────────┼──────────────────────────┤
│ 1.4 G (48%)      │ node_modules/            │
│ 820 M (28%)      │ target/                  │
│ 240 M (8%)       │ .git/                    │
│ 180 M (6%)       │ vendor/                  │
│ …                │ …                        │
└──────────────────┴──────────────────────────┘

/home/me/projects/pairee
Escaneados: 18 elementos · Total: 2.9 G · Profundidad: 2

Las 20 entradas más grandes:
```

Una notificación en la parte inferior reporta el total escaneado
y el límite top-N.

### Ajustar la profundidad de recursión

Pon `depth = 1` en el diálogo (`Opciones → Plugins → disk-usage`)
para medir sólo el **primer nivel** del cwd (un nivel de
recursión, sin contar archivos anidados). Útil para "¿qué
subcarpeta debería borrar para liberar más espacio?".

### Incluir archivos ocultos

Activa `include_hidden = true` si tu directorio home tiene
`~/.cache` o `~/.local/share` que quieras ver en el reporte.

### Pasar argumentos extra a `du`

Si tu versión de `du` soporta `--exclude`, puedes pasarlo a
través de `extra_args` para que se salten carpetas como
`build-artifacts/`:

```text
extra_args = "--exclude=build-artifacts"
```

## Resolución de problemas

| Síntoma | Causa probable | Solución |
|---------|----------------|----------|
| La notificación dice "Required tool 'du' is not on PATH" | El binario falta o el `$PATH` no incluye `/usr/bin` | Instala `coreutils` (Linux) o `du` viene preinstalado en macOS. En Windows, asegúrate de tener `du` (Cygwin / MSYS) — si no, debería activarse la rama de PowerShell. |
| El reporte está vacío (`No scannable entries were found in this directory`) | El cwd está vacío o pusiste `depth = 0` | Sube `depth` al menos a `1`, o cambia a una carpeta con archivos. |
| Los errores de permiso se descartan en silencio | El plugin ignora los archivos que no puede leer (evita spam en el reporte) | Vuelve a correr como administrador si necesitas esas entradas, o excluye la subcarpeta problemática con `extra_args`. |
| El plugin no carga | El hash de `main.lua` en el [files] del `manifest.toml` ya no coincide con el archivo en disco | Reinstala con `pairee plugin install disk-usage.pairee`. |
| El resultado es `0 B` para todas las entradas | `depth` es muy bajo para tu estructura, o el cwd está en un sistema de archivos distinto (p. ej. una unidad de red) | Prueba con `depth = 3` o más; en FS remotos, trabaja con una copia local. |
