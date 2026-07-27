# 🔧 How-To: Instalar, Compilar y Actualizar

> **Cuadrante: HOW-TO** — *orientado a problemas.*

Tres necesidades del día a día comparten esta página: **instalar**
Pairee por primera vez, **compilarlo** desde el código fuente, y
**actualizar** una instalación existente.

---

## Instalar con el script rápido (recomendado)

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh
```

El script:

1. Detecta tu plataforma y arquitectura.
2. Descarga el último release desde GitHub.
3. Verifica el hash SHA-256 contra el `*.sha256` del release.
4. Instala el binario (`/usr/local/bin/pairee` o
   `~/.local/bin/pairee`, lo que sea escribible).
5. Instala las carpetas `lang/`, `help/`, y `keymaps/` bajo
   `/usr/share/pairee/` (o `~/.local/share/pairee/`).

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1 | iex
```

El script de PowerShell refleja el de bash: descarga, verifica
SHA-256, instala en `%LOCALAPPDATA%\Programs\pairee`.

### Compilar desde fuente con el script

Si querés el último build no publicado (o no existe release para tu
plataforma), pasá el argumento `debug`:

```bash
curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh -s -- debug
```

Esto clona el repo, corre `cargo build`, e instala el binario
resultante.

### Desinstalar

```bash
curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh -s -- uninstall
```

O en Windows:

```powershell
irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1 | iex -Arguments uninstall
```

---

## Compilar desde el código fuente manualmente

### Prerrequisitos

- **Rust 1.70 o más nuevo** ([instrucciones de instalación](https://www.rust-lang.org/tools/install))
- Un toolchain de C (solo para algunas features opcionales)
- Git

### Pasos

```bash
# Clonar
git clone https://github.com/FittyAr/Pairee.git
cd Pairee

# Build debug (rápido, incluye símbolos de debug)
cargo build

# Build release (optimizado, sin logs de debug)
cargo build --release
```

El binario compilado queda en:

- `target/debug/pairee` (o `pairee.exe`)
- `target/release/pairee` (o `pairee.exe`)

### Correr desde una consola dedicada

Dos scripts de conveniencia abren Pairee en su propia ventana de
terminal:

- Linux / macOS: `./run.sh`
- Windows: `run.bat`

---

## Actualizar Pairee

### Auto-detectado en la app

Si `auto_update_check = true` (el default), Pairee consulta GitHub
Releases al arrancar. Cuando se publica un release nuevo, aparece un
badge amarillo **`▲ UPDATE`** en la esquina superior derecha.

1. Hacé clic en el badge (o `F9` → `Options` → `Check for updates`).
2. El diálogo de update muestra las release notes y el tamaño.
3. Pairee detecta **cómo se instaló** y aplica la acción correcta:

   | Método de instalación | Qué pasa |
   | --- | --- |
   | **tar.gz / ZIP binario directo** | Descarga el release nuevo, verifica SHA-256, reemplaza atómicamente el binario, pide reiniciar. |
   | **Windows Inno Setup installer** | Descarga el instalador, lo corre silenciosamente (`/VERYSILENT`), sale de Pairee. |
   | **`apt` / `dnf` / `pacman` / `nix`** | Muestra el comando exacto `sudo apt update && sudo apt install pairee` para que lo corras. |
   | **`winget` / `scoop` / `chocolatey`** | Muestra `winget upgrade Pairee` (o el equivalente de tu package manager). |
   | **`snap` / `flatpak`** | Muestra el comando correspondiente del package manager. |

4. El downloader siempre busca el `*.sha256` correspondiente y rehúsa
   instalar si el hash no matchea.

### Ignorar un release manualmente

Si querés quedarte en la versión actual a pesar de un release nuevo,
descartá el badge. Pairee escribe la versión ignorada en
`dismissed_update_version` en `settings.toml`. Limpiá ese campo para
re-habilitar las notificaciones de esa versión.

### Notas de diseño completas

Mirá [`52_explanation_update_system`](52_explanation_update_system.md)
para los 13 métodos de instalación que Pairee puede detectar y la
lógica detrás de la verificación SHA-256.

---

## Ubicaciones de configuración y datos

| Qué | Windows | Linux / macOS |
| --- | --- | --- |
| Config (temas, presets, historial) | `%APPDATA%\pairee\config\` | `~/.config/pairee/` |
| Cache (locks de plugins, badges de update) | `%APPDATA%\pairee\cache\` | `~/.cache/pairee/` |
| Log de debug | `%APPDATA%\pairee\cache\app.log` | `~/.cache/pairee/app.log` |

La primera vez que Pairee arranca, crea estos directorios y los
rellena con defaults razonables.

---

## A dónde ir ahora

- Por qué 13 métodos de instalación, no 1: [`52_explanation_update_system`](52_explanation_update_system.md)
- Referencia de configuración: [`41_reference_configuration`](41_reference_configuration.md)
