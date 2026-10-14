# recent-files

Un plugin mixto (hook + comando) que registra en segundo plano las carpetas
(y, si quieres, los archivos) que visitas y te permite volver a cualquiera
de ellas con una sola tecla.

## Cómo funciona

Mientras Pairee está en ejecución, el plugin:

1. Se suscribe a los eventos `on_cd` y (opcionalmente) `on_hover`.
2. Sube cada ruta visitada al principio del historial, guardando como
   máximo `max_entries` rutas.
3. Guarda el historial con `pairee.state`, en la carpeta de datos privada
   del plugin, así que se conserva entre sesiones.
4. Publica un evento pub/sub `recent-files:added` (`{ path, kind }`) para
   que otros plugins puedan reaccionar a la nueva visita.

## Comando

| Comando | Teclas por defecto | Acción |
|---------|--------------------|--------|
| Recent files | `Ctrl+Alt+e r` (Neovim `<leader>pr`, yazi `Alt+p r`) | Abre el selector de archivos recientes |

El comando también está en la paleta de comandos. Puedes reasignarlo en
`keybindings.toml` con su id, `plugin.recent-files.pick`.

El selector muestra las 35 entradas más recientes, cada una con una tecla
(`1`-`9` y luego `a`-`z`). Pulsa la tecla para saltar; `Esc` cancela. Una
carpeta se abre en el panel activo; para un archivo, el panel abre la
carpeta que lo contiene.

## Configuración

Abre el **Gestor de plugins**, selecciona el plugin y pulsa `c` (o Enter):

| Opción | Por defecto | Descripción |
|--------|-------------|-------------|
| `max_entries` | `50` | Número máximo de entradas recientes que se guardan. |
| `record_dirs` | `true` | Registrar los cambios de carpeta (`on_cd`). |
| `record_hover` | `false` | Registrar también cada archivo bajo el cursor. Desactivado por defecto: puede generar mucho ruido en carpetas grandes. |

## ¿Por qué no requiere confianza?

El plugin sólo usa `pairee.state` (su propia carpeta de datos) y los
eventos de los paneles. **No** lanza procesos ni lee otros archivos, así
que se ejecuta en modo no confiable (sandbox).

## Escucharlo desde otro plugin

```lua
pairee.ps.sub("recent-files:added", function(payload, meta)
    -- payload.path, payload.kind ("dir" o "file"); meta.sender == "recent-files"
end)
```

## Ejemplos

### El selector

Pulsa `Ctrl+Alt+e r`. Aparecerá un diálogo `pairee.which` con las entradas
recientes:

```text
1  [carpeta]  /home/me/projects/pairee
2  [archivo]  /home/me/projects/pairee/README.md
3  [carpeta]  /home/me/Downloads
```

Pulsa `2` para abrir `/home/me/projects/pairee`, donde está `README.md`.

## Resolución de problemas

| Síntoma | Causa probable | Solución |
|---------|----------------|----------|
| "Aún no hay archivos recientes — visita algunos directorios primero." | El historial está vacío (instalación nueva) | Visita algunas carpetas para que `on_cd` llene la lista. |
| No se registra nada | `record_dirs = false` en la configuración del plugin | Vuelve a activarlo en la configuración de `recent-files` del Gestor de plugins. |
| Nunca aparecen archivos | `record_hover` está desactivado (por defecto) | Activa `record_hover` para registrar también los archivos bajo el cursor. |
| El plugin no carga | Necesita Pairee 0.11.0 o posterior (API Lua 1.3.0) | Actualiza Pairee y reinstálalo con `pairee plugin install recent-files`. |
