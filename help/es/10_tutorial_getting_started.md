# 🎓 Tutorial: Primeros pasos con Pairee

> **Cuadrante: TUTORIAL** — *orientado al aprendizaje, hands-on, sin conocimiento previo.*

En los próximos diez minutos vas a instalar Pairee, lanzarlo y completar
una pequeña tarea real (copiar un árbol de carpetas, de forma segura).
Cada paso es concreto; si te trabás, saltá al documento de referencia
que está al final.

---

## 1. Instalar Pairee

### Oneliner (recomendado)

**Linux / macOS** (ejecutá en cualquier shell):

```bash
curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh
```

**Windows** (ejecutá en PowerShell):

```powershell
irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1 | iex
```

El script detecta tu plataforma, descarga el release correspondiente
desde GitHub, verifica el SHA-256 y coloca el binario en una ubicación
razonable (`/usr/local/bin` en Linux/macOS, `%LOCALAPPDATA%\Programs\pairee`
en Windows).

### Desde el código fuente

Si preferís compilar (o querés el último código):

```bash
git clone https://github.com/FittyAr/Pairee.git
cd Pairee
cargo build --release
./target/release/pairee         # o .\target\release\pairee.exe en Windows
```

Necesitás **Rust 1.70+** instalado. Los artefactos de compilación y las
dependencias de dev tardan ~3 min en una máquina moderna.

> ¿Necesitás desinstalar? Mirá
> [`29_howto_install_build_update`](29_howto_install_build_update.md).

---

## 2. Lanzar Pairee

Abrí una terminal y escribí:

```bash
pairee
```

Deberías ver una TUI con dos paneles, el **izquierdo** y el **derecho**
cada uno mostrando un listado de directorio, una **barra F-key** al
final (`1 Help  2 User  3 View  …`), una **barra de menú** arriba
(`Left | Files | Commands | Options | Right | Help`), y un **reloj** en
la esquina superior derecha.

> Si la terminal se ve mal, mirá la nota de troubleshooting al final.

---

## 3. El tour de 60 segundos

Probá cada uno de los siguientes en orden. Ninguno de estos pasos
modifica tu disco.

| Paso | Acción | Qué notar |
| --- | --- | --- |
| 1 | Apretá `Tab` | El foco salta al otro panel. |
| 2 | Apretá `Up` / `Down` (o `j` / `k`) | El resaltado se mueve; el otro panel queda intacto. |
| 3 | Apretá `F1` | Se abre el popup de ayuda con esta documentación. `Esc` lo cierra. |
| 4 | Apretá `Ctrl+1`, `Ctrl+2`, `Ctrl+3` | El panel activo cambia entre vista *Brief*, *Medium* y *Full*. |
| 5 | Apretá `F2` | Aparece un pequeño **User Menu** con comandos rápidos. |
| 6 | Apretá `F9` | El menú superior se despliega. Usá flechas + `Enter`, o presioná la letra resaltada (acelerador `&`). |
| 7 | Apretá `Ctrl+O` | Ambos paneles colapsan, exponiendo la pantalla cruda. Apretá de nuevo para traerlos de vuelta. |
| 8 | Apretá `F12` | El overlay **Screens** lista todas las pantallas abiertas. Apretá `Esc` para cerrarlo. |

Ahora sabés lo suficiente para manejar Pairee sin seguir leyendo. Las
siguientes secciones hacen una sola tarea realista.

---

## 4. Tu primera tarea real: copiar una carpeta de forma segura

Objetivo: copiar una carpeta llamada `~/projects/notes` a `~/backup/`.

1. **Navegá el panel izquierdo hasta el origen**:
   - Escribí una ruta: apretá `Shift+Tab` para enfocar la línea de
     comandos, escribí la ruta absoluta, apretá `Enter`.
   - O navegá: `Tab` para cambiar de panel, flechas para moverte,
     `Enter` para entrar a una carpeta, `Backspace` para subir un nivel.

2. **Navegá el panel derecho hasta el destino**:
   - Apretá `Tab` para enfocar el panel derecho, y repetí.

3. **Tageá la carpeta de origen**:
   - Mové el resaltado hasta `notes`.
   - Apretá `Insert` (o `Space`). El cursor salta al siguiente elemento
     y el archivo recibe un pequeño marcador de selección (el color
     depende de tu tema).

4. **Copiá**:
   - Apretá `F5` (o `Enter` sobre la etiqueta y después `F5`).
   - Un pequeño diálogo pregunta por el destino. Confirmá.

5. **Mirá la transferencia**:
   - Un popup de progreso muestra el archivo actual, velocidad de
     transferencia y ETA.
   - Podés seguir navegando los paneles mientras la copia corre en
     background. El popup de progreso queda encima hasta que termine.

6. **Verificá**:
   - Apretá `Ctrl+R` para refrescar el panel derecho.
   - La carpeta `backup/notes` está ahí. Podés apretar `F3` sobre ella
     para mirar adentro, `F4` para editar un archivo, `Enter` para
     entrar.

7. **Deshacer (si hace falta)**:
   - No hay undo global, pero podés usar `F8` para borrar lo que
     copiaste, y en el siguiente diálogo elegir **Move to Recycle Bin**
     (por defecto si la opción *Delete to Recycle Bin* está habilitada
     en [`41_reference_configuration`](41_reference_configuration.md),
     Tab 0).

🎉 Acabas de usar: foco de panel, navegación, tagueo, copia, progreso
async y refresh — el bucle del día a día.

---

## 5. A dónde ir ahora

| Si querés… | Leé |
| --- | --- |
| Aprender más sobre los dos paneles, modos de vista y pantallas | [`11_tutorial_panels_and_screens`](11_tutorial_panels_and_screens.md) |
| Encontrar un archivo específico o filtrar el panel | [`21_howto_search_filter_history`](21_howto_search_filter_history.md) |
| Mover/borrar/wipe/links/atributos | [`20_howto_file_operations`](20_howto_file_operations.md) |
| Empaquetar o desempaquetar archivos | [`22_howto_archives`](22_howto_archives.md) |
| Conectarte a un servidor remoto | [`23_howto_ssh_sftp`](23_howto_ssh_sftp.md) |
| Administrar un repositorio Git | [`24_howto_git_integration`](24_howto_git_integration.md) |
| Cambiar la apariencia (temas, layouts, F-key bar) | [`25_howto_appearance_themes`](25_howto_appearance_themes.md) |
| Cambiar los presets de keymap (Norton/Neovim/VSCode) | [`26_howto_keymaps_user_menu`](26_howto_keymaps_user_menu.md) |
| Instalar o escribir plugins | [`28_howto_plugins`](28_howto_plugins.md) |

---

## Troubleshooting

| Síntoma | Solución probable |
| --- | --- |
| La terminal se ve mal (líneas repintan mal) | Probá una terminal moderna: **Windows Terminal**, **WezTerm**, **Alacritty**, **kitty**. Desactivá ClearType vía `Configuration → Interface → ClearType friendly redraw` si estás en Windows Console host. |
| `pairee: command not found` | El script de instalación no puso el binario en el `PATH`. Re-ejecutalo, o agregá `~/.local/bin` (Linux/macOS) o `%USERPROFILE%\.cargo\bin` (Windows) a tu `PATH`. |
| La barra F-key se ve mal sobre SSH | Los modificadores `Ctrl` / `Alt` no viajan por SSH. Mirá [`30_howto_ssh_modifier_keys`](30_howto_ssh_modifier_keys.md). |
| Quiero mi viejo F1-F10 / menú F2 de vuelta | El keymap podría ser `neovim` o `vscode`. Mirá [`26_howto_keymaps_user_menu`](26_howto_keymaps_user_menu.md). |
| La pantalla parpadea o scrollea con cada repintado | `ratatui` requiere una terminal con alternate-screen. Todas las terminales de arriba lo soportan. Si estás dentro de `tmux`/`screen`, habilitá la opción `terminal-override` y usá `-2`. |
