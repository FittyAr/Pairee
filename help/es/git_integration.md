# Manual de Referencia de Integración con Git

Pairee cuenta con un panel de control de Git completamente integrado que le permite monitorear y gestionar el estado de su repositorio directamente dentro de la interfaz gráfica de terminal (TUI). Utiliza la biblioteca nativa libgit2 para proporcionar operaciones rápidas, seguras y asíncronas.

---

## 1. Acceso a Git y Operaciones de Explorador

### 1.1 Atajos del Panel
* **Atajo de Teclado:** Presione **`Alt+G`** (o **`Alt+g`**) dentro de cualquier directorio perteneciente a un repositorio Git.
* **Menú Superior:** Seleccione **`Panel Izquierdo`** (o **`Panel Derecho`**) -> **`Panel Git`**.

### 1.2 Gestión de Repositorios desde los Paneles de Navegación
* **Inicializar Repositorio (`GitInit`):** Al ubicarse en una carpeta sin repositorio, el menú ofrece **`Inicializar repositorio Git`** para crear un nuevo repositorio en dicha ubicación.
* **Clonar Repositorio (`GitClone`):** Disponible en el menú superior (**`Clonar repositorio...`**). Solicita la URL remota (HTTPS o SSH) y el nombre de la carpeta de destino.
* **Detección Automática:** Cuando `git_auto_detect` está activado en la configuración, Pairee escaneará automáticamente buscando carpetas `.git` al navegar.

### 1.3 Indicadores Visuales en los Paneles de Archivos
* **Rama en el Título:** El encabezado del panel muestra la rama activa actual (ej. `[git: main]`, `[git: feature/xyz]` o `[git: (detached HEAD)]`).
* **Distintivos de Estado de Archivos:** Los archivos modificados en el árbol de trabajo se distinguen con insignias y colores:
  - `[M]` (Amarillo): Archivo modificado.
  - `[A]` (Verde): Archivo agregado / preparado (staged).
  - `[?]` (Magenta): Archivo sin rastrear (untracked).
  - `[D]` (Rojo): Archivo eliminado.
  - `[!]` (Rojo Claro): Archivo en conflicto de fusión.

---

## 2. Pestañas Interactivas del Panel de Git

El panel de Git cuenta con cinco pestañas independientes. Utilice las teclas **`Tab`** o **`Shift+Tab`** para alternar entre ellas.

### 2.1 Pestaña de Estado (Status)
Muestra todos los archivos modificados, preparados, desfasados y en conflicto en su árbol de trabajo.
* **Indicadores de Estado:**
  - `[staged]`: Cambios preparados en el índice listos para commit.
  - `[staged+]`: Archivo con cambios en el índice y modificaciones adicionales sin preparar.
  - `M` (Modificado), `A` (Agregado), `D` (Eliminado), `?` (Sin rastrear), `R` (Renombrado), `!` (Conflicto).
* **Comandos de Teclado:**
  - **`Espacio`**: Alterna la preparación del archivo seleccionado (prepara o desprepara).
  - **`a`**: Prepara todos los cambios y archivos nuevos (`git add -A`).
  - **`A`**: Desprepara todos los archivos del índice.
  - **`x` / `Delete`**: Descarta cambios en el archivo seleccionado (solicita confirmación).
  - **`i`**: Agrega el archivo o patrón seleccionado al archivo `.gitignore`.
  - **`c`** (Commit): Abre el diálogo para confirmar cambios preparados.
    - Dentro del diálogo: **`Ctrl+A`** alterna **Amend** (`--amend`) para modificar el commit anterior.
  - **`d`**: Abre el visor Git Diff para inspeccionar diferencias en el archivo seleccionado.
  - **`s`**: Guarda los cambios en la pila de stash (solicita mensaje; **`Ctrl+U`** incluye archivos untracked).
  - **`X`**: Aborta una fusión en progreso si existen conflictos.
  - **`f` / `l` / `u`**: Fetch, Pull o Push de sincronización remota.
  - **`Esc`**: Cierra el panel de Git.

### 2.2 Pestaña de Historial (Log)
Muestra el historial de commits de la rama activa con paginación continua y scroll infinito.
* **Columnas de Metadatos:**
  - **Commit Hash:** Identificador hexadecimal corto de 7 caracteres.
  - **Fecha:** Fecha del commit formateada como `AAAA-MM-DD`.
  - **Autor:** Nombre del autor.
  - **Mensaje:** Primera línea del mensaje del commit.
* **Comandos de Teclado:**
  - **`Enter`**: Hace checkout del commit en modo **HEAD desasociada** (con confirmación).
  - **`d`**: Abre el visor de diff con los cambios del commit.
  - **`b` / `n`**: Crea una nueva rama apuntando al commit seleccionado.
  - **`t`**: Crea una nueva etiqueta apuntando al commit seleccionado.
  - **`c`**: Realiza cherry-pick del commit hacia la rama actual (con confirmación).
  - **`r`**: Revierte el commit creando un commit inverso (con confirmación).
  - **`y`**: Copia el hash SHA completo (40 caracteres) al portapapeles.
  - **`s`**: Reset Soft al commit seleccionado (conserva índice y directorio).
  - **`x`**: Reset Mixed al commit seleccionado (reinicia índice, conserva directorio).
  - **`h`**: Reset Hard al commit seleccionado (descarta todos los cambios).
  - **`Esc`**: Cierra el panel de Git.

### 2.3 Pestaña de Ramas (Branches)
Lista todas las ramas locales y remotas con contadores de commits ahead/behind.
* **Indicadores Visuales:**
  - Rama activa marcada con `*` y resaltada.
  - Ramas remotas etiquetadas con `[remote]` en gris.
  - Indicadores ahead/behind: `[↑X ↓Y]` indica commits adelantados y atrasados respecto al upstream.
* **Comandos de Teclado:**
  - **`Enter`**:
    - En rama local: cambia a dicha rama.
    - En rama remota: realiza checkout creando automáticamente una rama local con upstream vinculado.
  - **`n`**: Crea una nueva rama a partir de HEAD.
  - **`d` / `Delete`**: Elimina la rama seleccionada (locales o remotas, con confirmación).
  - **`r`**: Renombra la rama local seleccionada.
  - **`b`**: Rebasa (rebase) la rama actual sobre la seleccionada (con confirmación).
  - **`m`**: Fusiona (merge) la rama seleccionada en la rama actual (con confirmación).
  - **`R`**: Abre el diálogo de **Gestión de Repositorios Remotos**.
  - **`Esc`**: Cierra el panel de Git.

### 2.4 Pestaña de Stash
Muestra todos los estados guardados en la pila de stash.
* **Comandos de Teclado:**
  - **`Enter` / `a`**: Aplica los cambios del stash seleccionado al directorio de trabajo.
  - **`p`**: Pop del stash (aplica los cambios y lo elimina de la pila).
  - **`d`**: Abre el visor de diff de los cambios contenidos en el stash.
  - **`Delete` / `x`**: Elimina la entrada de stash seleccionada (con confirmación).
  - **`C`**: Limpia todas las entradas de la pila de stash (con confirmación).
  - **`Esc`**: Cierra el panel de Git.

### 2.5 Pestaña de Etiquetas (Tags)
Muestra todas las etiquetas del repositorio (tanto ligeras como anotadas).
* **Columnas de Metadatos:**
  - **Nombre de Etiqueta:** Nombre de referencia de la etiqueta.
  - **Commit:** Hash del commit de destino.
  - **Anotación:** Mensaje descriptivo de la etiqueta anotada (si existe).
* **Comandos de Teclado:**
  - **`Enter`**: Realiza checkout de la etiqueta en modo HEAD desasociada (con confirmación).
  - **`n`**: Crea una nueva etiqueta en el commit HEAD (solicita nombre).
  - **`d` / `Delete`**: Elimina la etiqueta seleccionada (con confirmación).
  - **`u`**: Empuja (push) todas las etiquetas al repositorio remoto.
  - **`Esc`**: Cierra el panel de Git.

---

## 3. Operaciones Remotas y Autenticación

### 3.1 Sincronización Remota
Desde cualquier pestaña del panel:
* **`f`**: Fetch de cambios desde el remoto upstream.
* **`l`**: Pull de cambios (fetch + merge / fast-forward) desde la rama upstream.
* **`u`**: Push de commits locales al remoto (configura `--set-upstream` automáticamente si es necesario).

### 3.2 Diálogo de Gestión de Remotos (`R`)
Al presionar **`R`** en la pestaña de Ramas:
* Visualice todos los remotos configurados junto con sus URLs de fetch y push.
* **`a`**: Agregar un nuevo remoto (solicita nombre y URL).
* **`d` / `Delete`**: Eliminar el remoto seleccionado (con confirmación).

### 3.3 Soporte de Autenticación
* **Claves SSH:** Negociación automática con el agente SSH activo y claves estándar `~/.ssh/id_ed25519`, `~/.ssh/id_ecdsa` y `~/.ssh/id_rsa`.
* **HTTPS:** Integración completa con asistentes de credenciales del sistema (Git Credential Manager en Windows, Keychain en macOS y libsecret en Linux).
