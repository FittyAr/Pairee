# disk-usage

Un plugin de comandos para Pairee que analiza el uso de disco del directorio
de trabajo del panel activo y muestra qué elementos son los más pesados.

## Comando

| Comando | Teclas por defecto | Acción |
|---------|--------------------|--------|
| Disk usage report | `Ctrl+Alt+e d` (Neovim `<leader>pd`, yazi `Alt+p d`) | Ejecuta el análisis de uso de disco sobre la carpeta del panel activo |

El comando también está en la paleta de comandos. Puedes reasignarlo en
`keybindings.toml` con su id, `plugin.disk-usage.report`.

El resultado se muestra en la Vista rápida de la carpeta escaneada como una
tabla ordenada (los más grandes primero) y una notificación reporta cuántos
elementos se escanearon. Los paneles remotos (SFTP, FTP, ...) no se escanean.

## Configuración

Abre el **Gestor de plugins**, selecciona el plugin y pulsa `c` (o Enter):

| Opción | Por defecto | Descripción |
|--------|-------------|-------------|
| `depth` | `2` | Hasta qué profundidad lista `du` las subcarpetas (`1` = sólo el primer nivel). En Windows siempre se lista el primer nivel. |
| `top_n` | `20` | Cuántas de las entradas más grandes mostrar. |
| `include_hidden` | `false` | Incluir archivos ocultos / directorios con punto. |
| `extra_args` | `""` | Argumentos extra añadidos a la invocación de `du` (solo Linux / macOS). |

## Cómo funciona

- En **Linux / macOS** lanza `du -k -d <depth> <cwd>` (`-d` funciona con `du` de GNU y de BSD).
- En **Windows** recorre el directorio de forma nativa con
  `pairee.fs.read_dir` y suma los tamaños por entrada de nivel superior (no
  sigue enlaces simbólicos). No lanza ningún proceso.

La salida se parsea, se ordena por tamaño, se recorta a las N entradas más
grandes y se muestra con `pairee.preview_widget` como un resumen sobre una
tabla `pairee.ui.Table`.

## ¿Por qué es confiable?

El plugin necesita lanzar `du` (Linux / macOS), por lo que se ejecuta en
modo **confiable** (trusted). Se te pedirá que confíes en él la primera
vez que lo instales.

En **Modo Seguro** solo pueden ejecutarse programas declarados;
`manifest.toml` declara:

```toml
[permissions]
commands = ["du"]
```

## Ejemplos

### Ejecución por defecto (depth=2, top_n=20)

Dentro de una carpeta de proyecto típica, pulsa `Ctrl+Alt+e d`. La Vista
rápida de la carpeta muestra un reporte similar a:

```text
/home/me/projects/pairee
Escaneados: 18 elementos · Total: 2.9 GB
Los 18 más grandes:

Tamaño            Ruta
1.4 GB (48%)      /home/me/projects/pairee/node_modules
820.0 MB (28%)    /home/me/projects/pairee/target
240.0 MB (8%)     /home/me/projects/pairee/.git
180.0 MB (6%)     /home/me/projects/pairee/vendor
…                 …
```

Una notificación reporta el total escaneado y cuántas entradas se
muestran.

### Ajustar la profundidad de recursión

Pon `depth = 1` en la configuración del plugin (Gestor de plugins → `disk-usage` → `c`)
para listar sólo el **primer nivel** de la carpeta (cada entrada
sigue contando todo lo que tiene debajo). Útil para "¿qué
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
| La notificación dice "La herramienta 'du' requerida no está en el PATH" | El binario falta o el `$PATH` no incluye `/usr/bin` | Instala `coreutils` (Linux) o `du` viene preinstalado en macOS. Windows no necesita `du` (escaneo nativo). |
| El reporte está vacío (`No se encontraron elementos escaneables en este directorio`) | El cwd está vacío o pusiste `depth = 0` | Sube `depth` al menos a `1`, o cambia a una carpeta con archivos. |
| Los errores de permiso se descartan en silencio | El plugin ignora los archivos que no puede leer (evita spam en el reporte) | Vuelve a correr como administrador si necesitas esas entradas, o excluye la subcarpeta problemática con `extra_args`. |
| El plugin no carga | Necesita Pairee 0.11.0 o posterior (API Lua 1.3.0) | Actualiza Pairee y reinstálalo con `pairee plugin install disk-usage`. |
| El resultado es `0 B` para todas las entradas | `depth` es muy bajo para tu estructura, o el cwd está en un sistema de archivos distinto (p. ej. una unidad de red) | Prueba con `depth = 3` o más; en FS remotos, trabaja con una copia local. |
