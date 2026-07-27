# 📖 Referencia: Campos del Diálogo SSH y Operaciones SFTP

> **Cuadrante: REFERENCE** — *orientado a información.*

Referencia rápida para el diálogo de conexión SSH y las operaciones
SFTP disponibles en un panel conectado. Para el workflow, mirá
[`23_howto_ssh_sftp`](23_howto_ssh_sftp.md).

---

## 1. Campos del diálogo de conexión

| Campo | Tipo | Notas |
| --- | --- | --- |
| **Preset Name** | string | Nickname opcional. Requerido si querés guardar un bookmark. |
| **Host** | string | IP o dominio. |
| **Port** | integer | Default `22`. |
| **Username** | string | Login remoto. |
| **Password** | string | Password plana **o** la passphrase para desbloquear la private key. |
| **Key Path** | path | Ruta absoluta a una private key (ej. `~/.ssh/id_ed25519`). Dejar en blanco para password o agent auth. |

El diálogo también expone tres botones:

| Botón | Efecto |
| --- | --- |
| **Connect** | Abre la conexión. |
| **Save** | Guarda los campos actuales como bookmark (requiere un Preset Name). |
| **Load** | Llena los campos desde el bookmark resaltado. |
| **Delete** | Quita el bookmark resaltado. |

> La columna izquierda lista todos los bookmarks guardados. Usá las
> flechas para resaltar, después **Load** para llenar los campos, y
> después **Connect**.

---

## 2. Estados de la conexión

| Estado | Barra de título | Qué funciona |
| --- | --- | --- |
| Desconectado | `<local path>` | Todo. |
| Conectando | `Connecting to user@host:port…` | Solo el botón de cancelación. |
| Conectado | `[SSH: user@host]` | Todas las operaciones SFTP. |
| Falló | `Connection failed: <razón>` | Nada. Cerrá el diálogo y reintentá. |

---

## 3. Operaciones SFTP

Una vez que el panel activo está en modo SFTP (título `[SSH: ...]`),
las siguientes acciones de F-key se remapean al filesystem
**remoto**:

| Acción | Efecto en remoto |
| --- | --- |
| `Enter` | Abre la carpeta resaltada o corre la file association matcheante. |
| `Backspace` | Va al directorio padre. |
| `F3` | Ver (descarga un buffer temporal, abre en el visor interno). |
| `F4` | Editar (descarga un buffer temporal, abre en el editor interno, sube al guardar). |
| `F5` | Copia al panel opuesto (download si el opuesto es local, upload si el opuesto es SFTP, server-to-server si ambos son SFTP). |
| `F6` | Mueve. |
| `F7` | Renombra. |
| `F8` | Borrado recursivo en remoto. |
| `Insert` / `Space` | Taguea para operaciones en bulk. |
| `Gray+` / `Gray-` / `Gray*` | Selección en bulk por glob / unselect / invertir. |
| `Ctrl+R` | Re-lee el listado remoto. |
| `Ctrl+\` | Abre el diálogo de Folder Shortcuts (funciona del lado local, no del remoto). |

> El panel de Transfers (`Ctrl+T`) lista cada transferencia en curso
> y completada. El popup de progreso muestra los mismos datos que para
> copias locales: archivo, velocidad, ETA, total.

---

## 4. Formato del archivo de bookmarks

Los bookmarks se guardan como una lista TOML bajo `ssh_bookmarks` en
`settings.toml`:

```toml
[[ssh_bookmarks]]
name = "Production"
host = "ssh.example.com"
port = 22
user = "deploy"
key_path = "/home/me/.ssh/id_ed25519"
# la password NO se persiste intencionalmente
```

> Pairee **no** guarda passwords en el archivo de config. Cuando
> recargues un bookmark, dejá el campo de password vacío o llenalo de
> nuevo.

---

## 5. Manejo de host keys

- En la **primera** conexión a un host, Pairee muestra el fingerprint
  de la host key y te pregunta si confiás. Los fingerprints aceptados
  se guardan en `known_hosts` (junto al `~/.ssh/known_hosts` del
  sistema).
- Si la key de un host **cambia**, Pairee rechaza la conexión y
  pregunta si querés actualizar. Confirmá solo si estás seguro de que
  el server fue reinstalado.
- Algoritmos de fingerprint soportados: `SHA256:...` (OpenSSH moderno)
  y la forma legacy separada por dos puntos de MD5, para
  retrocompatibilidad.

---

## 6. Métodos de autenticación

| Método | Cuándo lo usa Pairee |
| --- | --- |
| **Password** | Tipeaste un valor en el campo `Password` y dejaste `Key Path` en blanco. |
| **Public key (file)** | `Key Path` está seteado y el archivo es legible. El campo `Password` se trata como la **passphrase** para desbloquear la key. |
| **SSH agent** | `Password` vacío, `Key Path` vacío, y un agente del sistema es alcanzable. |
| **Keyboard-interactive** | Cae automáticamente cuando el server lo pide. |

> Formatos de public key aceptados: `RSA`, `ECDSA`, `Ed25519`. La
> legacy `DSA` **no** se acepta.

---

## 7. Mensajes de error comunes

| Error | Causa | Fix |
| --- | --- | --- |
| `Connection refused` | Host o puerto incorrecto, o el daemon SSH no está corriendo. | Verificá host/puerto. Chequeá `systemctl status sshd` en el server. |
| `Permission denied (publickey)` | El server rechazó la key. | Chequeá `~/.ssh/authorized_keys` en el server. Verificá permisos de archivos (`chmod 600`). |
| `Host key verification failed` | La key del host cambió. | Confirmá que el server fue reinstalado, después aceptá la nueva key. |
| `Connection timed out` | Firewall o routing. | Abrí el puerto en el firewall del server; chequeá tu ISP. |
| `No supported auth methods` | El server no tiene un método de auth que Pairee pueda usar. | Habilitá `PasswordAuthentication` o `PubkeyAuthentication` en `sshd_config`. |

---

## A dónde ir ahora

- Workflow SSH: [`23_howto_ssh_sftp`](23_howto_ssh_sftp.md)
- Teclas modificadoras sobre SSH: [`30_howto_ssh_modifier_keys`](30_howto_ssh_modifier_keys.md)
