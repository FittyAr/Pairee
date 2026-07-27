# 💡 Explicación: El Sistema de Update y los 13 Métodos de Instalación

> **Cuadrante: EXPLANATION** — *orientado a entender.*

El auto-updater de Pairee es más que un loop de "chequear GitHub y
descargar". Detecta **cómo instalaste Pairee** y aplica la estrategia
de update correcta. Esta página explica la detección, el chequeo de
integridad, y por qué existen trece métodos de instalación.

---

## 1. La fase de detección

Cuando Pairee arranca, el módulo `update::detect` intenta identificar
el método de instalación en este orden:

| # | Método | Cómo se detecta | Acción de update |
| --- | --- | --- | --- |
| 1 | **tar.gz binario directo** (Linux) | El path del binario vive bajo `~/.local/bin/pairee` o `/usr/local/bin/pairee`, ningún package manager lo posee. | Descarga el tarball nuevo, reemplazo atómico del binario, pide reiniciar. |
| 2 | **ZIP binario directo** (Windows) | El path del binario vive bajo `%LOCALAPPDATA%\Programs\pairee\pairee.exe` (o similar), sin instalador MSI/EXE en registry. | Descarga el ZIP nuevo, escribe un helper `.bat` auto-destructible, sale de Pairee, el helper reemplaza el binario y relanza. |
| 3 | **Inno Setup installer** (Windows) | Pairee está registrado en "Agregar/Quitar Programas" con `Inno Setup` como publisher, o una key de registry llamada "Inno Setup:". | Descarga el instalador, lo corre con `/VERYSILENT`, sale de Pairee. |
| 4 | **`apt`** (Debian, Ubuntu, derivados) | `dpkg -S $(which pairee)` devuelve un paquete `.deb`; existe `/var/lib/dpkg/info/pairee.list`. | Muestra el comando `sudo apt update && sudo apt install pairee`. |
| 5 | **`dnf` / `yum`** (Fedora, RHEL, CentOS) | `rpm -qf $(which pairee)` devuelve un paquete. | Muestra el comando `sudo dnf upgrade pairee`. |
| 6 | **`pacman`** (Arch, Manjaro) | `pacman -Qo $(which pairee)` devuelve el paquete. | Muestra el comando `sudo pacman -Syu pairee`. |
| 7 | **`zypper`** (openSUSE) | `zypper se --installed-only pairee` devuelve el paquete. | Muestra el comando `sudo zypper update pairee`. |
| 8 | **`nix`** (NixOS, nixpkgs) | El path del binario está dentro de `/nix/store/`. | Muestra el comando `nix-env -u pairee` o `nixos-rebuild switch`. |
| 9 | **`snap`** | `snap list pairee` devuelve un paquete. | Muestra el comando `sudo snap refresh pairee`. |
| 10 | **`flatpak`** | `flatpak list | grep pairee` devuelve un paquete. | Muestra el comando `flatpak update pairee`. |
| 11 | **`winget`** (Windows) | `winget list | grep pairee` devuelve un paquete. | Muestra el comando `winget upgrade FittyAr.Pairee`. |
| 12 | **`scoop`** (Windows) | `scoop list | grep pairee` devuelve un paquete. | Muestra el comando `scoop update pairee`. |
| 13 | **`chocolatey`** (Windows) | `choco list | grep pairee` devuelve un paquete. | Muestra el comando `choco upgrade pairee`. |

Si ninguno de los anteriores matchea, Pairee asume el path de
**binario directo** y ofrece descargar un tarball/ZIP en el directorio
del binario.

> La detección pasa una vez por launch y el resultado se cachea en
> memoria. Podés re-correr la detección manualmente desde
> `F9` → `Options` → `Check for updates`.

---

## 2. El chequeo de integridad

Cada release descargado viene con un archivo **`.sha256`** al lado del
archivo binario. Pairee:

1. Descarga el archivo binario nuevo.
2. Descarga el `.sha256` matcheante del mismo release.
3. Computa el SHA-256 del archivo descargado localmente.
4. Compara los dos hashes.

Si difieren, el archivo se **borra inmediatamente** y el update se
aborta. Un toast de notificación le avisa al usuario que la
verificación falló. El `app.log` local registra el mismatch para
debug.

Esto protege contra:

- Un upload de GitHub Releases comprometido.
- Un MITM en el network path (aunque HTTPS ya previene esto; SHA-256
  es la segunda línea de defensa).
- Una descarga corrupta (raro, pero posible en redes flaky).

> El paso de verificación pasa **antes** de que se reemplace cualquier
> archivo en disco. Una verificación fallida nunca deja tu sistema
> en un estado parcial.

---

## 3. El auto-check

Si `auto_update_check = true` (el default), Pairee consulta GitHub
Releases una vez al arrancar. La consulta es non-blocking y está
throttled a una vez por hora por sesión. El resultado se cachea para
el resto de la corrida.

Cuando se encuentra un release más nuevo, aparece un badge amarillo
**`▲ UPDATE`** arriba a la derecha de la barra F-key. Hacé clic (o
usá `F9` → `Options` → `Check for updates`) para abrir el diálogo de
update.

### El diálogo de update

El diálogo muestra:

- La versión nueva.
- La fecha del release.
- Las release notes (rendereadas desde el body del Release de GitHub
  como Markdown).
- El tamaño del binario y el SHA-256 (click para copiar).
- Un botón **Download and apply** (o el comando del package manager
  que el usuario debería correr).

Si el usuario hace clic en **Download and apply** (o el botón del
package manager es la única opción), Pairee recorre el path elegido:

- **Binario directo**: descargar, verificar, reemplazar, pedir
  reiniciar.
- **Inno Setup**: descargar, correr silenciosamente, salir.
- **Package manager**: solo mostrar el comando; el usuario lo pega
  en su shell.

### Descartar un update

Si el usuario descarta el badge, Pairee escribe el string de versión
en `dismissed_update_version` en `settings.toml`. Los launches
futuros no re-notifican para esa versión. Para re-habilitar las
notificaciones, limpiá el campo a mano.

---

## 4. Por qué trece métodos, no uno

Un approach único de "descargar y reemplazar" está bien para usuarios
que instalaron Pairee vía el script de install, pero es **incorrecto**
para usuarios en distros package-managed:

- Un update de binario directo pasa por encima del package manager,
  así que `apt` / `dnf` / `pacman` van a reportar el archivo como
  modificado y pueden sobreescribirlo en el próximo update del
  sistema.
- En **NixOS**, los archivos bajo `/nix/store/` son immutables;
  intentar reemplazarlos silenciosamente va a fallar.
- En **Windows**, un EXE instalado tiene un path de update diferente
  (instalador MSI/EXE) que un ZIP portable.
- **Snap** y **Flatpak** sandboxes prohíben escribir en sus mount
  points; los updates tienen que ir por el manager del sandbox.

Mostrarle al usuario el **comando correcto para su método de
instalación** es más seguro y más rápido que intentar ser piola en su
nombre.

---

## 5. El flow de update, end-to-end

```
launch
  └─▶ update::detect         (averiguar método de instalación)
        └─▶ GitHub Releases   (si auto_update_check)
              └─▶ newer?     (comparar semver)
                    └─▶ sí   → renderizar badge
                              └─▶ user clicks badge
                                    └─▶ update::downloader
                                          ├─▶ descargar archive
                                          ├─▶ descargar .sha256
                                          ├─▶ verificar hash
                                          └─▶ update::installer
                                                ├─▶ direct: reemplazar + reiniciar
                                                ├─▶ Inno Setup: correr instalador silenciosamente
                                                └─▶ package manager: mostrar comando
```

---

## 6. Dónde mirar en el código

- `src/update/detect.rs` — detección del método de instalación.
- `src/update/downloader.rs` — descarga + verificación SHA-256.
- `src/update/installer.rs` — aplicar el update correcto.
- `src/update/checker.rs` — consultar GitHub Releases.

---

## A dónde ir ahora

- Workflow de update: [`29_howto_install_build_update`](29_howto_install_build_update.md)
- Configuración: [`41_reference_configuration`](41_reference_configuration.md)
