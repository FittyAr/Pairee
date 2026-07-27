# 🔧 How-To: Integración con Git

> **Cuadrante: HOW-TO** — *orientado a problemas.*

Pairee incluye un dashboard Git completo que opera sobre el repositorio
que contiene la ruta del panel activo. Está construido sobre `libgit2`
y corre sus operaciones en workers de background, así que la UI nunca
se bloquea por red o disco.

---

## Abrir el dashboard Git

| Disparador | Pasos |
| --- | --- |
| Hotkey | `Alt+G` (o `Alt+g`) |
| Menú | `Left` (o `Right`) → `Git` (solo se muestra si la ruta activa está dentro de un repo) |
| Auto-detect | Si `git_auto_detect` está habilitado en `Configuration → Git`, Pairee escanea hacia arriba el árbol de directorios para encontrar la raíz del repo mientras navegás. |

Se abre un modal con **cuatro pestañas**. Usá `Tab` / `Shift+Tab` para
ciclar.

---

## Pestaña 1: Status

Lista todos los archivos en el working tree que difieren de `HEAD`,
con un prefijo de una letra:

| Prefijo | Color | Significado |
| --- | --- | --- |
| `M` | Amarillo | **Modificado** en el working tree. |
| `A` | Verde | **Agregado** al index. |
| `D` | Rojo | **Borrado** del working tree. |
| `?` | Gris oscuro | **Untracked** (nuevo, no está en `.gitignore`). |
| `R` | Cyan | **Renombrado**. |
| `!` | Magenta | **En conflicto** (unmerged). |

| Tecla | Efecto |
| --- | --- |
| `Space` | Alterna staging del archivo resaltado. |
| `c` | **Commit** de todos los cambios stageados (abre el diálogo de commit). |
| `d` | **Diff** del archivo resaltado contra `HEAD`/index. |
| `s` | **Stash** de los cambios actuales (pide un mensaje opcional). |
| `r` | **Refresh** de la lista de status. |
| `Esc` | Cierra el dashboard. |

### Flujo de commit

1. Apretá `Space` en cada archivo que querés incluir en el commit
   (solo los archivos stageados se commitean).
2. Apretá `c`.
3. Escribí un mensaje de commit. Pairee escribe el mensaje en el
   buffer `COMMIT_EDITMSG` del repo, así que el hook de editor (si
   hay) aplica.
4. Confirmá.

> Los mensajes vacíos se rechazan.

### Visor de diff

Apretá `d` en cualquier fila de status. El diff se abre en el visor
interno con resaltado rojo/verde. Apretá `F4` adentro del diff para
alternar texto/hex (rara vez útil para un diff) o `F7` para buscar.

---

## Pestaña 2: Log

Muestra el historial de commits de la rama actual, del más nuevo al
más viejo.

| Columna | Significado |
| --- | --- |
| Hash | Primeros 7 hex chars del SHA del commit. |
| Date | `YYYY-MM-DD` en tu zona horaria local. |
| Author | El nombre del autor (configurado por `git config user.name`). |
| Message | Primera línea del mensaje de commit. |

| Tecla | Efecto |
| --- | --- |
| `Enter` | **Checkout** del commit resaltado (te pone en **detached HEAD**; te pedirá confirmar). |
| `d` | Muestra el **diff** introducido por este commit. |
| `s` | **Soft reset** al commit resaltado (mantiene los cambios stageados). |
| `x` | **Mixed reset** (mantiene los cambios en el working tree, los unstage). |
| `h` | **Hard reset** (descarta los cambios). |
| `r` | Refresca el log. |

> La cantidad de entradas de log que se muestran está limitada por
> **Max log entries** en `Configuration → Git`. El default es razonable
> para la mayoría de los repos; subilo si trabajás con historiales
> muy largos.

---

## Pestaña 3: Branches

Lista ramas locales y ramas remote-tracking.

- La rama actual está marcada con un `*` verde.
- Las ramas remote-tracking están etiquetadas con `[remote]` y
  rendereadas en gris.

| Tecla | Efecto |
| --- | --- |
| `Enter` | **Checkout** de la rama local resaltada. |
| `n` | **Nueva** rama (pide un nombre). |
| `d` / `Delete` | **Borra** la rama local resaltada (pide confirmación; la rama actual no se puede borrar). |
| `r` | **Renombra** la rama local resaltada. |
| `m` | **Merge** de la rama resaltada en la actual (pide confirmación). |
| `r` | Refresca. |

---

## Pestaña 4: Stash

Lista las entradas de `git stash list`.

| Tecla | Efecto |
| --- | --- |
| `a` | **Apply** del stash resaltado (lo mantiene en el stack). |
| `p` / `Enter` | **Pop** del stash resaltado (aplica y lo borra). |
| `d` / `Delete` | **Drop** de la entrada de stash resaltada. |

---

## Operaciones remotas (cualquier pestaña)

| Tecla | Efecto |
| --- | --- |
| `f` | **Fetch** desde el remote. |
| `l` | **Pull** (fetch + merge) desde la rama remota activa. |
| `u` | **Push** de los cambios commiteados a la rama remota activa. |

> Pairee usa la config de Git en tu repo (`.git/config` y
> `~/.gitconfig`). Para sobreescribir la identidad del autor para una
> sesión, configurá **Author name** y **Author email** en
> `Configuration → Git`.

---

## Errores comunes

- **El dashboard no se abre**: el panel activo no está dentro de un
  repositorio Git. Navegá un nivel arriba (`Backspace`) y probá de
  nuevo, o habilitá `git_auto_detect`.
- **Push rechazado**: el remote tiene commits nuevos que vos no
  tenés. Apretá `l` para hacer pull (o rebase) primero, resolvé
  cualquier conflicto, después `u`.
- **Conflictos en stash al aplicar**: Pairee muestra los archivos en
  conflicto en la pestaña Status; resolvé manualmente y
  `git add`, después commit.

---

## A dónde ir ahora

- Configuración: [`41_reference_configuration`](41_reference_configuration.md) (Tab 6: Git Settings)
- Workers de background: [`50_explanation_architecture`](50_explanation_architecture.md)
