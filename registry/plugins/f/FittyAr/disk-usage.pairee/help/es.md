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
