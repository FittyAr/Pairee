# recent-files.pairee

Un plugin mixto (hook + comando) que registra en segundo plano los
archivos y directorios que visitas, y te permite volver a cualquiera de
ellos con una sola pulsación.

## Cómo funciona

Mientras Pairee está en ejecución, el plugin:

1. Se suscribe a los eventos `on_cd` y (opcionalmente) `on_hover`.
2. Registra cada visita con su timestamp.
3. Escribe la lista (con debounce) en un archivo JSON dentro del
   directorio de configuración de Pairee (por defecto,
   `recent-files.json`).
4. Publica un evento `recent-files:added` para que otros plugins
   puedan reaccionar (por ejemplo, un plugin "abrir en editor" puede
   mantener su propia lista MRU sin duplicar estado).

## Atajo de teclado

| Tecla | Acción |
|-------|--------|
| `Ctrl+R` | Abre el selector de archivos recientes en el panel activo |

El selector es un prompt `which`: escribes el número de la entrada (o
usas las flechas) y `Enter` para saltar. `Esc` cancela.

## Configuración

| Opción | Por defecto | Descripción |
|--------|-------------|-------------|
| `max_entries` | `50` | Número máximo de entradas recientes que se conservan. |
| `record_dirs` | `true` | Registrar cambios de directorio (`on_cd`) además de selecciones de archivo. |
| `record_hover` | `false` | Registrar también cada hover del cursor. Apagado por defecto — puede ser ruidoso en directorios grandes. |
| `persist_path` | `""` | Sobrescribe la ruta del archivo JSON. Vacío = usar la ruta por defecto de Pairee. |

## API pública para otros plugins

Otros plugins pueden llamar a este a través del límite `pairee.sync`:

```lua
local recent = require("recent-files.pairee")
local items  = recent:list(5)   -- 5 entradas más recientes
```

El plugin también publica un evento `recent-files:added`:

```lua
pairee.ps.sub("recent-files:added", function(entry)
    -- entry.path, entry.kind, entry.at
end)
```

## ¿Por qué no requiere confianza?

El plugin solo usa la API de FS de Pairee (`pairee.fs.read` /
`pairee.fs.write`) para persistir el estado. **No** lanza procesos
externos, así que puede ejecutarse sin problemas en modo no confiable
(sandbox).
