# 🔧 How-To: Conexiones Remotas SSH y SFTP

> **Cuadrante: HOW-TO** — *orientado a problemas.*

Pairee trae un cliente SSH integrado y un backend SFTP. Una vez
conectado, el panel activo se vuelve un navegador de archivos remoto, y
podés copiar entre local y remoto con el mismo `F5` / `F6` que ya usás
para archivos locales.

---

## Abrir el diálogo de conexión

Hay tres formas de lanzar el diálogo SSH:

| Disparador | Pasos |
| --- | --- |
| Hotkey | `Ctrl+Shift+S` |
| Menú | `Left` (o `Right`) → `Connect to SSH…` |
| Menú de drives | `Alt+F1` (izq) / `Alt+F2` (der) → elegí `[Connect SSH]` |

Aparece un modal con los campos de conexión a la derecha y tus
bookmarks guardados a la izquierda.

---

## Conectarte por primera vez

1. Completá los campos de conexión:

   | Campo | Ejemplo | Notas |
   | --- | --- | --- |
   | Preset Name | `Production API` | Nickname opcional, usado para bookmarks. |
   | Host | `ssh.example.com` o `192.168.1.50` | |
   | Port | `22` | Puerto SSH por defecto. |
   | Username | `deploy` | |
   | Password | `••••••` | O la passphrase para desbloquear tu key. |
   | Key Path | `/home/me/.ssh/id_ed25519` | Dejar en blanco para usar password o agente SSH. |

2. Hacé clic en **`Connect`** (o apretá `Enter`).

3. En la **primera** conexión, Pairee mostrará la host key y te
   preguntará si confiás en ella. Verificá el fingerprint y aceptá.

4. Cuando se establece la conexión, el **panel activo** se vuelve un
   navegador remoto. La barra de título muestra
   `[SSH: username@host]`.

---

## Marcar la conexión como bookmark (preset)

1. Después de completar los campos, escribí un **Preset Name** único
   (ej. `Work staging`).
2. Hacé clic en **`Save`**.
3. El preset se guarda en tu `settings.toml` y aparece en la lista
   izquierda la próxima vez que abras el diálogo.

Para **cargar** un preset guardado: elegilo en la lista izquierda,
hacé clic en `Load`, después `Connect`.

Para **borrar** un preset: elegilo, hacé clic en `Delete`.

---

## Navegar el panel remoto

| Tecla | Efecto |
| --- | --- |
| `Enter` | Abre la carpeta resaltada, o corre las asociaciones sobre un archivo. |
| `Backspace` | Sube al directorio padre. |
| `Ctrl+R` | Re-lee el listado remoto. |
| `F3` | Ver un archivo remoto (texto o hex). |
| `F4` | Editar un archivo remoto en el editor interno (usa un buffer local temporal y sube al guardar). |
| `F7` | Renombrar un archivo remoto. |
| `F6` | Mover archivos remotos entre directorios. |
| `F8` | Borrado recursivo en el remoto. |
| `Alt+F1` / `Alt+F2` | Menú de drives (usalo para cambiar a un disco local). |

---

## Transferir archivos local ↔ remoto

Los dos paneles actúan independientes: uno puede ser local, el otro
SFTP (o ambos locales, o ambos SFTP a dos hosts distintos).

### Upload (local → remoto)

1. Enfocá el panel **local**; tagueá los archivos a subir.
2. Asegurate de que el panel **remoto** muestre la carpeta destino.
3. Apretá `F5` (copia) o `F6` (mueve).

### Download (remoto → local)

1. Enfocá el panel **remoto**; tagueá los archivos.
2. Asegurate de que el panel **local** muestre la carpeta destino.
3. Apretá `F5` (copia) o `F6` (mueve).

La transferencia corre en un worker de background. Un popup de
progreso muestra:

- Archivo actual
- Bytes por segundo
- Transcurrido / ETA
- Total de bytes
- Una barra de progreso global

Podés cambiar de pantalla mientras la transferencia está corriendo.

> El botón `Copy` del diálogo SSH **no transfiere archivos**; copia
> los campos del diálogo al portapapeles. Usá `F5` / `F6` para mover
> los archivos reales.

---

## Desconectar

| Disparador | Pasos |
| --- | --- |
| Menú | `Left` (o `Right`) → `Disconnect SSH` |
| Menú de drives | `Alt+F1` / `Alt+F2` → elegí cualquier drive local (ej. `/` o `C:`) |

El panel activo vuelve a una vista de disco local.

---

## Errores comunes

- **Host key cambiada**: Pairee rechaza la conexión y pregunta si
  querés actualizar. Hacelo solo si estás seguro de que el server
  fue reinstalado.
- **Permisos del key incorrectos**: en Linux/macOS, el cliente SSH
  rechaza keys legibles por otros. `chmod 600 ~/.ssh/id_*`.
- **Sin password / agente**: dejá `Password` vacío **y** `Key Path`
  vacío para que el agente SSH del sistema (si hay) maneje la
  autenticación.
- **Time zone / locale**: los timestamps y listados de archivos se
  renderean en UTC por default; Pairee sigue la zona horaria local
  para mostrarlos.

---

## A dónde ir ahora

- Referencia de campos del diálogo: [`44_reference_ssh_fields`](44_reference_ssh_fields.md)
- Teclas modificadoras sobre SSH: [`30_howto_ssh_modifier_keys`](30_howto_ssh_modifier_keys.md)
- Transferencias en background: [`50_explanation_architecture`](50_explanation_architecture.md)
