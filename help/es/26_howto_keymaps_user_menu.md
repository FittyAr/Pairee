# 🔧 How-To: Keymaps y el User Menu

> **Cuadrante: HOW-TO** — *orientado a problemas.*

Pairee trae **tres presets de keymap** y te permite remapear cualquier
tecla en un archivo `user.toml`. El **User Menu** (`F2`) es una capa
separada, liviana, para los comandos que más corrés.

---

## Cambiar el preset de keymap

Los tres presets bundled viven en la carpeta `keymaps/`, al lado del
ejecutable (o en `/usr/share/pairee/keymaps/` en Linux):

| Preset | Inspirado en | Mejor para |
| --- | --- | --- |
| `norton.toml` | Norton Commander, Far Manager | Usuarios clásicos de doble panel. |
| `neovim.toml` | Neovim / Oil.nvim / NvimTree | Amantes de lo modal, `h/j/k/l`. |
| `vscode.toml` | VS Code Explorer | Desarrolladores con memoria muscular de Ctrl. |

### Elegir un preset

1. Abrí `settings.toml` (en tu carpeta de config de Pairee).
2. Seteá el campo `keymap`:

   ```toml
   keymap = "neovim"
   ```

3. Guardá y reiniciá Pairee. La barra F-key y todos los keybindings
   reflejan el nuevo preset inmediatamente.

> Los valores válidos matchean los filenames de los presets sin la
> extensión. Mirá [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
> para una comparación lado a lado de los tres presets.

### Creá tu propio preset

1. Copiá cualquiera de los archivos `.toml` bundled a un archivo nuevo
   dentro del directorio `keymaps/`.
2. Editá la tabla `[bindings]`. Cada línea es `action = "KeyCombo"`.
   Se permiten múltiples bindings para la misma acción (separadas por
   coma dentro del string).
3. Referenciá el preset por el stem del filename en `settings.toml`:

   ```toml
   keymap = "mi_custom"
   ```

Para la lista completa de acciones, mirá
[`43_reference_actions`](43_reference_actions.md).

### Overlay de user keymap

Si solo querés **sobreescribir unas pocas teclas** sin escribir un
preset completo, creá `keymaps/user.toml`. Las teclas en `user.toml`
toman precedencia sobre el preset activo, y no tenés que mantener
una copia completa.

---

## El User Menu (F2)

`F2` abre un pequeño overlay con una lista numerada de comandos
rápidos. Los items por defecto son:

| # | Etiqueta | Acción |
| --- | --- | --- |
| 1 | Refresh | `Ctrl+R` |
| 2 | Toggle hidden | `Ctrl+H` |
| 3 | Swap panels | `Ctrl+U` |
| 4 | Task list | `Ctrl+W` |
| 5 | Git panel | `Alt+G` |
| 6 | Make folder | (Diálogo MkDir) |
| F | Quick filter | Filtro de substring en vivo |
| H | Help overlay | `F1` |
| E | Edit user menu | Abre el editor |

> Apretá la **letra resaltada** para disparar la acción. `Esc` cierra
> el menú.

### Personalizar el User Menu

La entrada "Edit user menu" (y apretar `E` en el menú) abre un editor
TOML para `usermenu.toml` en tu carpeta de config. El archivo mapea
una tecla a **una acción con label** o a una **plantilla de comando
shell**.

```toml
[[items]]
key = "1"
label = "Refresh"
action = "refresh"            # o un comando custom en su lugar

[[items]]
key = "2"
label = "Git status (one-liner)"
command = "git -C %p status -sb"
# %p = ruta del panel actual, %f = archivo resaltado, %% = literal %
```

Cuando el menú tiene al menos una línea `command = "..."`, Pairee usa
tu menú custom en lugar del default.

> La entrada `E` (Edit) siempre se agrega al final así podés
> reabrir el editor.

---

## Folder shortcuts (Ctrl+Alt+1 … 9)

Podés bindear hasta **nueve rutas** a `Ctrl+Alt+1` hasta `Ctrl+Alt+9`.
Setup:

1. Apretá `Ctrl+\` para abrir el diálogo **Folder shortcuts**.
2. Elegí un slot (`Insert` para agregar nueva entrada, `e` para
   editar, `Delete` para quitar).
3. Escribí la ruta. `Enter` para guardar.

Para **saltar** a un atajo guardado, apretá la tecla
`Ctrl+Alt+N` correspondiente.

---

## A dónde ir ahora

- Lista completa de acciones: [`43_reference_actions`](43_reference_actions.md)
- Tablas lado a lado de atajos: [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
- Ciclado de modificadores sobre SSH: [`30_howto_ssh_modifier_keys`](30_howto_ssh_modifier_keys.md)
