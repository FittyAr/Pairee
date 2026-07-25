# Manual de Referencia de Integración con Git

Pairee cuenta con un panel de control de Git completamente integrado que le permite monitorear y gestionar el estado de su repositorio directamente dentro de la interfaz gráfica de terminal (TUI). Utiliza la biblioteca nativa de Git para proporcionar operaciones rápidas, seguras y asíncronas.

---

## 1. Abrir el Panel de Git

Para iniciar el panel de control de Git, su panel de archivos activo debe estar dentro de una carpeta que forme parte de un repositorio Git válido.
* **Atajo de Teclado:** Presione **`Alt+G`** (o **`Alt+g`**).
* **Menú Desplegable:** Seleccione **`Panel Izquierdo`** (o **`Panel Derecho`**) -> **`Git`** desde el menú superior.
* **Detección Automática:** Si `git_auto_detect` está activado en la configuración, Pairee escaneará automáticamente los directorios buscando carpetas `.git` mientras navega.

---

## 2. Pestañas Interactivas del Panel de Git

El panel de Git muestra cuatro pestañas distintas. Utilice la tecla **`Tab`** (o **`Shift+Tab`**) para alternar entre ellas.

### 2.1 Pestaña de Estado (Status)
Esta pestaña muestra todos los archivos modificados, preparados (staged) y no rastreados en su directorio de trabajo.
* **Indicadores de Prefijo de Archivo:**
  - `M` (Amarillo) ⋄ **Modificado:** El archivo ha sido modificado en el directorio de trabajo.
  - `A` (Verde) ⋄ **Agregado:** El archivo es nuevo y está preparado (staged) en el índice.
  - `D` (Rojo) ⋄ **Eliminado:** El archivo ha sido eliminado del repositorio.
  - `?` (Gris Oscuro) ⋄ **Sin rastrear:** El archivo es nuevo y aún no está rastreado por Git.
  - `R` (Cian) ⋄ **Renombrado:** El archivo ha sido renombrado.
  - `!` (Magenta) ⋄ **Conflicto:** El archivo tiene conflictos de fusión sin resolver.
* **Comandos de Teclado:**
  - **`Space`**: Alterna la preparación del archivo seleccionado (prepara archivos modificados, quita del stage los preparados).
  - **`c`** (Commit): Abre el diálogo de Commit para confirmar los cambios preparados.
  - **`d`**: Abre el visor de Git Diff para inspeccionar los cambios en el archivo seleccionado.
  - **`s`**: Guarda los cambios actuales en la pila de stash (solicita un mensaje opcional).
  - **`r`** (Actualizar): Vuelve a leer las listas de estado activas.
  - **`Esc`**: Cierra el panel de Git.

### 2.2 Pestaña de Historial (Log)
Muestra un historial de commits detallado de la rama activa, desde `HEAD` hasta el límite configurado.
* **Columnas de Metadatos Mostradas:**
  - **Commit Hash:** Identificador hexadecimal corto de 7 caracteres.
  - **Fecha:** Marca de tiempo del commit formateada como `AAAA-MM-DD`.
  - **Autor:** El nombre del desarrollador que realizó el commit.
  - **Mensaje:** La primera línea del mensaje de commit.
* **Comandos de Teclado:**
  - **`Enter`** (Checkout Commit): Realiza el checkout del commit resaltado, colocando su repositorio en estado de **HEAD desasociada**. Se mostrará un diálogo de confirmación primero.
  - **`d`**: Abre el diff del commit para inspeccionar los cambios introducidos por este commit.
  - **`s`**: Realiza un **Reset Soft** al commit seleccionado.
  - **`x`**: Realiza un **Reset Mixed** al commit seleccionado.
  - **`h`**: Realiza un **Reset Hard** al commit seleccionado.
  - **`r`** (Actualizar): Vuelve a leer el registro de commits.

### 2.3 Pestaña de Ramas (Branches)
Lista todas las ramas locales y remotas disponibles en el repositorio.
* **Indicadores Visuales:**
  - La rama activa actualmente está marcada con un asterisco verde (`*`).
  - Las ramas de seguimiento remoto están etiquetadas con el prefijo `[remote]` y se renderizan en gris.
* **Comandos de Teclado:**
  - **`Enter`** (Checkout Rama): Cambia HEAD a la rama local seleccionada.
  - **`n`**: Solicita un nombre para crear una nueva rama partiendo de HEAD.
  - **`d` / `Delete`**: Elimina la rama local seleccionada (requiere confirmación; la rama actual no se puede eliminar).
  - **`r`**: Solicita un nuevo nombre para renombrar la rama local seleccionada.
  - **`m`**: Fusiona (merge) la rama seleccionada en la rama actual (requiere confirmación).
  - **`r`** (Actualizar): Recarga el listado de ramas.

### 2.4 Pestaña de Stash
Muestra todos los stashes guardados en la pila del repositorio.
* **Comandos de Teclado:**
  - **`a`**: Aplica los cambios de la entrada de stash seleccionada en su directorio de trabajo.
  - **`p` / `Enter`**: Ejecuta pop sobre el stash (aplica los cambios y elimina la entrada de la pila).
  - **`d` / `Delete`**: Elimina la entrada de stash de la pila de forma permanente.

---

## 3. Operaciones Remotas

Desde cualquier pestaña del Panel de Git, puede realizar sincronizaciones remotas:
* **`f`**: Fetch de cambios desde el repositorio remoto.
* **`l`**: Pull de cambios (fetch + merge) de la rama remota activa.
* **`u`**: Push de los commits locales preparados a la rama remota.
