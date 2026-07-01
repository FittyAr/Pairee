# Manual de Conexiones SSH y SFTP

Pairee integra un cliente SSH y un motor de protocolo SFTP completo, permitiéndote gestionar carpetas y archivos en servidores remotos Unix/Linux o Windows a través de SSH directamente desde uno de los paneles de navegación tradicionales.

---

## 1. Establecer una Conexión

Existen tres métodos para abrir el cuadro de diálogo de conexión SSH:
1. **Atajo de Teclado:** Presiona **`Ctrl+Shift+S`**.
2. **Menú Desplegable:** Selecciona **`Files`** (o **`Commands`**) -> **`Connect SSH...`** en la barra superior.
3. **Panel de Selección de Unidad:** Presiona **`Alt+F1`** (panel izquierdo) o **`Alt+F2`** (panel derecho) para abrir la lista de discos, y selecciona la opción **`[Connect SSH]`**.

---

## 2. Parámetros del Diálogo de Conexión

El formulario de conexión contiene los siguientes parámetros de ajuste:
* **Preset Name (Nombre del Preset):** Una etiqueta identificativa personalizada para esta conexión (ej. `Servidor de Producción API`). Si la introduces, podrás guardarla como marcador.
* **Host:** Dirección IP o dominio del servidor remoto (ej. `192.168.1.50` o `ssh.ejemplo.com`).
* **Port (Puerto):** Puerto SSH del servidor remoto (por defecto es `22`).
* **Username (Usuario):** Nombre de usuario de acceso en el servidor remoto.
* **Password (Contraseña):** Contraseña del usuario para autenticación estándar, O la frase de paso (passphrase) para descifrar el archivo de llave privada.
* **Key Path (Ruta de la Llave):** Ruta absoluta local a tu archivo de clave privada SSH (ej. `/home/usuario/.ssh/id_rsa`). Déjala en blanco para usar contraseña o el agente local SSH.

---

## 3. Marcadores de SSH (Presets)

* **Guardar Marcadores:** Rellena los datos de conexión, escribe un **Preset Name** único y presiona **`[Save]`**. El marcador se guardará en la configuración de la aplicación.
* **Cargar Marcadores:** La columna izquierda muestra los marcadores guardados. Usa las flechas de dirección o haz clic para seleccionar uno, pulsa **`[Load]`** para completar los datos y haz clic en **`[Connect]`** (o presiona `Enter`) para iniciar la sesión.
* **Eliminar Marcadores:** Selecciona un preset en la lista y presiona **`[Delete]`**.

---

## 4. Navegar por Sistemas de Archivos Remotos (SFTP)

Una vez establecida la conexión, el panel activo pasa a modo SFTP:
* El título del panel se actualiza de forma dinámica a: `[SSH: usuario@host]`.
* **Navegación Básica:**
  - **`Enter`**: Abre el directorio seleccionado o ejecuta las asociaciones de archivos.
  - **`Backspace`** o **`..`**: Vuelve al directorio de nivel superior.
* **Operaciones sobre Archivos:**
  - **`F7`** (MkDir): Crea una nueva carpeta en el servidor remoto.
  - **`F6`** (Rename/Move): Renombra o traslada archivos en caliente dentro del servidor.
  - **`F8`** (Delete): Elimina de forma recursiva carpetas y archivos en el servidor remoto.
  - **`F3`** (Viewer): Visualiza contenidos de archivos remotos en texto plano o modo hexadecimal.
  - **`F4`** (Editor): Edita archivos de texto directamente en el servidor remoto. Pairee gestiona los búferes temporales automáticamente.

---

## 5. Transferencia Bidireccional de Archivos

Puedes copiar o mover archivos entre tu sistema local y el servidor remoto de forma asíncrona:
* **Subir (Upload):** Enfoca tu panel local, marca los archivos que deseas enviar y presiona **`F5`** (Copiar) o **`F6`** (Mover) para subirlos a la carpeta remota del panel opuesto.
* **Descargar (Download):** Enfoca el panel de SSH (SFTP), marca los archivos remotos y presiona **`F5`** (Copiar) o **`F6`** (Mover) para descargarlos a la carpeta local activa del panel opuesto.
* **Cola en Segundo Plano:** Todas las transferencias ocurren en hilos de trabajo asíncronos. Una ventana flotante de progreso mostrará el nombre del archivo actual, velocidad de transferencia (MB/s), tiempo transcurrido, tamaño total y una barra de progreso. Puedes cambiar de pantalla mientras las tareas completan su curso en segundo plano.

---

## 6. Desconexión

Para cerrar la sesión SSH y restaurar el panel al sistema de archivos local:
1. Abre el menú superior: **`Files`** (o **`Commands`**) -> **`Disconnect SSH`**.
2. O bien, abre el selector de unidad (`Alt+F1` / `Alt+F2`) y elige cualquier punto de montaje local (ej. `/` o `C:`).
