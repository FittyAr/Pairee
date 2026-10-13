# Manual de Conexiones Remotas (SFTP, FTP, FTPS, WebDAV, S3)

Pairee puede mostrar un servidor remoto en cualquiera de los paneles y copiar entre dos lugares cualesquiera: local ↔ remoto y remoto ↔ remoto (por ejemplo, de un servidor FTP directamente a un bucket S3). Los protocolos son SFTP (SSH), FTP, FTPS (FTP con TLS explícito), WebDAV (HTTP o HTTPS) y S3 (Amazon S3 y almacenes compatibles como MinIO, Garage, Ceph o Cloudflare R2).

---

## 1. Abrir el administrador de conexiones

1. **Atajo de teclado:** pulse **`Ctrl+Shift+S`**.
2. **Menú:** **`Archivos`** (o **`Comandos`**) → **`Conectar a un servidor...`**.

La conexión se abre en el panel que estaba activo.

---

## 2. El diálogo de conexión

La columna izquierda lista las conexiones guardadas con su protocolo, p. ej. `backup (SFTP)`. Use las flechas (o haga clic) para mostrar una en los campos de la derecha. `Tab` / `Mayús+Tab` recorren la lista, los campos y los botones.

| Campo | Significado |
| :--- | :--- |
| **Nombre** | Una etiqueta para la conexión guardada. |
| **Protocolo** | SFTP, FTP, FTPS, WebDAV, WebDAV (HTTPS) o S3. Se cambia con `←` / `→` o `Espacio`. Un puerto que era el predeterminado del protocolo anterior pasa al del nuevo. |
| **Host** (**Bucket** en S3) | El nombre o la IP del servidor; en S3, el bucket. |
| **Puerto** | Vacío usa el predeterminado del protocolo (22, 21, 80 o 443). S3 no lo usa. |
| **Usuario** (**Clave de acceso** en S3) | El nombre de inicio de sesión. FTP y WebDAV entran de forma anónima sin él; S3 usa entonces `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY`. |
| **Contraseña** (**Clave secreta** en S3) | Se guarda en el llavero del sistema (ver §5). Puede quedar vacía: Pairee la pide al conectar. En SFTP es también la frase de contraseña del archivo de clave. |
| **Ruta de clave** | Solo SFTP: su archivo de clave privada. Vacío: se prueban el ssh-agent y las claves predeterminadas de `~/.ssh`. |
| **Carpeta** | La carpeta que se abre al conectar (vacío: la raíz del servidor). En WebDAV y S3 es la raíz del panel (un prefijo de claves en S3). |
| **Opciones S3** | Solo S3: `endpoint=https://minio.local:9000&region=eu-west-1&path_style=true`. Sin endpoint se usa AWS. |

Botones: **Conectar**, **Guardar** (guarda los campos con su nombre y reemplaza una conexión del mismo nombre), **Eliminar** y **Cancelar**. Conectar no guarda.

---

## 3. Contraseñas y frases de contraseña pedidas cuando hacen falta

Si una conexión necesita un secreto que Pairee no tiene, se abre un pequeño diálogo de **Autenticación** mientras la conexión sigue en segundo plano:

* un servidor SFTP que acepta contraseñas cuando no funcionó ninguna clave, agente ni contraseña;
* la frase de contraseña de una clave SSH cifrada (el archivo configurado o una clave predeterminada de `~/.ssh`);
* la contraseña de una conexión FTP, WebDAV o S3 con usuario pero sin contraseña guardada, o cuya contraseña fue rechazada;
* fetch, pull y push de Git con una clave cifrada en `~/.ssh`.

Lo que escribe se oculta. `Intro` lo envía y `Esc` cancela (la conexión se detiene con un mensaje). Los secretos escritos ahí se usan solo para ese inicio de sesión; guárdelos en la conexión para conservarlos.

---

## 4. Trabajar en un panel remoto

El título del panel muestra el protocolo y el servidor, p. ej. `[FTP: usuario@host]`.

* **Navegación:** `Intro` abre carpetas, `Retroceso` / `..` sube.
* **F5 / F6 (Copiar / Mover):** entre un panel remoto y uno local, o entre dos paneles remotos de cualquier protocolo. Las transferencias corren en segundo plano con el progreso del panel de transferencias, preguntas de conflicto, reintentos, verificación y reanudación. Mover dentro de una misma conexión es un renombrado en el servidor (y se puede deshacer).
* **F7 (Renombrar), Crear carpeta, F8 (Eliminar):** en el servidor (eliminar no se puede deshacer).
* **F3 / F4:** ver, o editar mediante una copia local temporal que se sube al guardar (antes se comprueba si cambió en el servidor).
* **Ctrl+A (Atributos):** solo SFTP.
* La actualización automática consulta carpetas remotas solo con `auto_refresh_ssh` activado.

---

## 5. Dónde se guardan los secretos

Las contraseñas y las claves secretas de S3 se guardan en el llavero del sistema (Administrador de credenciales de Windows, Llavero de macOS, Secret Service en Linux), nunca en `config.toml`. Si el llavero no se puede usar se conservan solo durante la sesión, salvo que ponga `passwords_in_config = true` en la sección `[security]` de `config.toml`. Si el llavero no se puede leer al iniciar (bloqueado, no disponible momentáneamente), la contraseña guardada no se toca y se vuelve a leer al conectar.

Las conexiones guardadas por versiones anteriores (`[[ssh_presets]]`) se leen como conexiones SFTP y sus entradas del llavero se mueven automáticamente.

---

## 6. Desconectar

**`Archivos`** (o **`Comandos`**) → **`Desconectar`** devuelve el panel al disco local, igual que elegir una unidad local en el menú de unidades (`Alt+F1` / `Alt+F2`).
