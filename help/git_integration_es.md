# Manual de Integración con Git

Pairee cuenta con un panel de Git completamente integrado que permite supervisar y gestionar el estado de tu repositorio directamente desde la interfaz del terminal. Utiliza llamadas nativas a las librerías de Git para ofrecer operaciones rápidas, seguras y asíncronas.

---

## 1. Abrir el Panel de Git

Para abrir el panel interactivo de Git, el panel de archivos activo debe encontrarse apuntando a una carpeta que forme parte de un repositorio Git válido.
* **Atajo de Teclado:** Presiona **`Alt+G`** (o **`Alt+g`**).
* **Menú Superior:** Selecciona **`Panel Izquierdo`** (o **`Panel Derecho`**) -> **`Git`**.
* **Autodetectar repositorios:** Si la opción `git_auto_detect` está activada en la configuración, Pairee escaneará las carpetas en busca de archivos `.git` a medida que navegas.

---

## 2. Pestañas Interactivas del Panel de Git

El panel emergente de Git consta de tres pestañas diferenciadas. Usa la tecla **`Tab`** para alternar entre ellas.

### 2.1 Pestaña Status (Estado)
Muestra los archivos modificados, agregados (staged), eliminados y sin seguimiento (untracked) de tu directorio de trabajo.
* **Indicadores de Prefijo de Archivos:**
  - `M` (Amarillo) ⋄ **Modified (Modificado):** El archivo ha sido modificado en el área de trabajo.
  - `A` (Verde) ⋄ **Added (Agregado):** El archivo es nuevo y ha sido preparado en el índice (staged).
  - `D` (Rojo) ⋄ **Deleted (Eliminado):** El archivo ha sido eliminado del repositorio.
  - `?` (Gris Oscuro) ⋄ **Untracked (Sin seguimiento):** El archivo es nuevo y no está rastreado por Git.
  - `R` (Cian) ⋄ **Renamed (Renombrado):** El archivo ha sido renombrado.
  - `!` (Magenta) ⋄ **Conflicted (En Conflicto):** El archivo tiene conflictos de fusión sin resolver.
* **Comandos de Teclado:**
  - **`c`** (Commit All): Prepara todos los archivos modificados y sin seguimiento (`git add -A`) y abre el cuadro de diálogo de Commit.
  - **`r`** (Refresh): Vuelve a escanear el repositorio para actualizar la lista de estados.
  - **`Esc`**: Cierra el panel de Git.

### 2.2 Pestaña Log (Historial)
Muestra la lista de commits recientes desde `HEAD` en la rama activa hasta el límite configurado.
* **Columnas de Metadatos:**
  - **Hash de Commit:** Identificador hexadecimal corto de 7 caracteres.
  - **Fecha:** Marca de tiempo formateada como `YYYY-MM-DD`.
  - **Autor:** El nombre del desarrollador que realizó el commit.
  - **Mensaje:** La primera línea del mensaje de commit.
* **Comandos de Teclado:**
  - **`Enter`** (Checkout Commit): Realiza un checkout del commit seleccionado, cambiando el repositorio a un estado de **HEAD desprendido** (detached HEAD). Requiere confirmación previa.
  - **`r`** (Refresh): Recarga el historial de commits.

### 2.3 Pestaña Branches (Ramas)
Lista todas las ramas locales y ramas de seguimiento remoto en el repositorio.
* **Indicadores Visuales:**
  - La rama activa actual se resalta y se marca con un asterisco verde (`*`).
  - Las ramas de seguimiento remoto tienen el prefijo `[remote]` y se renderizan en gris.
* **Comandos de Teclado:**
  - **`Enter`** (Checkout Branch): Cambia HEAD a la rama local seleccionada y actualiza los archivos del panel de trabajo. Requiere confirmación previa.
  - **`r`** (Refresh): Actualiza la lista de ramas.

---

## 3. Ventana de Commit e Identidad de Autor

Al realizar un commit de los cambios desde la pestaña `Status`:
1. Presiona **`c`**. Pairee ejecutará `git add -A` de forma interna.
2. Aparecerá un cuadro de diálogo: **"Commit All Changes"** (Confirmar Todos los Cambios).
3. Introduce el mensaje del commit. Si se deja vacío, se cancelará la operación.
4. Pairee resolverá tu identidad de la siguiente manera:
   - Consulta los parámetros de configuración `git_author_name` y `git_author_email`.
   - Si están vacíos, busca en la configuración local del repositorio (`.git/config`) o global del sistema (`~/.gitconfig`).
   - Si no encuentra configuración de Git en el sistema, utiliza por defecto: `Pairee User <pairee@localhost>`.
5. Presiona **`Enter`** para realizar el commit, o **`Esc`** para cancelar.
