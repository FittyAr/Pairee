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

## Ejemplos

### El estado en disco

`recent-files.json` vive en el directorio de configuración de Pairee
y se ve así:

```json
{
  "_recent_files_v1": true,
  "entries": [
    { "path": "/home/me/projects/pairee", "kind": "dir",  "at": 1722115200 },
    { "path": "/home/me/projects/pairee/README.md", "kind": "file", "at": 1722111600 },
    { "path": "/home/me/Downloads", "kind": "dir",  "at": 1722024000 }
  ]
}
```

La entrada más nueva siempre está primera. El formato lleva el
sentinel `_recent_files_v1` para que una migración futura pueda
detectar y actualizar archivos antiguos.

### El selector

Pulsa `Ctrl+R` con el cursor sobre cualquier panel. Obtendrás un
prompt `pairee.which` con todas las entradas recientes:

```text
[1]  /home/me/projects/pairee
[2]  /home/me/projects/pairee/README.md
[3]  /home/me/Downloads
[4]  /tmp/scratch.txt
```

Escribe el número (o usa las flechas) y pulsa `Enter` para saltar.
`Esc` cancela.

## Resolución de problemas

| Síntoma | Causa probable | Solución |
|---------|----------------|----------|
| La notificación dice "No recent files tracked yet — visit a few directories first" | El archivo de estado está vacío (recién instalado) o perdió el sentinel `_recent_files_v1` | Visita algunas carpetas para que `on_cd` pueble la lista. Si el archivo fue editado a mano y perdió el sentinel, bórralo — el plugin lo recreará en el próximo `on_cd`. |
| El selector se abre pero salta a la carpeta equivocada | Dos entradas recientes comparten un padre y elegiste la equivocada | Vuelve a abrir el selector y lee el índice con más cuidado. El plugin elige por *índice*, no por nombre. |
| No se está registrando nada | `record_dirs = false` en la configuración del plugin | Vuélvelo a `true` desde `Opciones → Plugins → recent-files`. El valor por defecto es `true`, pero un usuario previo puede haberlo desactivado. |
| El uso de disco crece sin límite porque el historial se agranda | `max_entries` es muy alto o `record_hover = true` en un directorio grande | Baja `max_entries` (por defecto: 50), o desactiva `record_hover`. El plugin no escribe en cada pulsación — usa debounce — pero un archivo de 100k entradas sigue siendo mucho que parsear. |
| El plugin no carga | El hash de `main.lua` en el [files] del `manifest.toml` ya no coincide con el archivo en disco | Reinstala con `pairee plugin install recent-files.pairee`. |
| Otro plugin no puede suscribirse a `recent-files:added` | El plugin que se suscribe se cargó antes que este | Los plugins deben cargarse en orden de dependencia. `recent-files.pairee` debe aparecer en `Plugins → Instalados` antes que cualquier plugin que se suscriba a su evento pub/sub. |
