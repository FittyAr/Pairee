# Plan de Refactor del Sistema de Transferencia Unificado

> **Objetivo:** expandir el sistema de transferencia refactorizado (hoy solo local) para cubrir **todas** las variantes entre paneles (local ↔ local, local ↔ SSH, SSH ↔ SSH mismo servidor, SSH ↔ SSH servidores distintos) y absorber **todas** las operaciones de archivos (copiar, mover, renombrar, eliminar, wipe, crear enlace, sincronización) en un único motor, con manejo completo de symlinks, permisos, ACL, xattr, Alternate Data Streams, resolución de conflictos, verificación, pausa, cancelación, progreso y reportes. Al final **todos los modales viejos deben quedar eliminados** y cada acción de archivo debe pasar por el nuevo motor.

---

## 1. Estado actual (diagnóstico)

### 1.1 Lo que YA está bien
| Componente | Ubicación | Estado |
|---|---|---|
| `TransferJob` / `TransferEngine` / `TransferQueue` | `fs/transfer/{job,engine,queue}.rs` | ✅ Reutilizable, agnóstico del medio físico |
| `TransferOptions` con `verify_after_copy`, `preserve_timestamps`, `preserve_attributes`, `preserve_acl`, `preserve_streams`, `skip_symlinks`, `follow_symlinks`, `max_retries`, `conflict_resolution`, `filter_mask`, `limit_bandwidth_rate`, `halt_on_error`, `delete_to_recycle_bin` | `fs/transfer/options.rs` | ✅ Completo |
| `TransferEvents` (full event bus) | `fs/transfer/events.rs` | ✅ |
| `preserve_metadata` para local (timestamps, permisos, atributos Windows, ACL Windows, ADS) | `fs/transfer/metadata.rs` | ✅ Local completo, **falta xattr Unix** y **falta remoto** |
| `copy_file_pipelined` con reader/writer en paralelo, hash streaming, bandwidth throttling, direct I/O con fallback | `fs/transfer/pipeline.rs` | ✅ Pero **solo local** (usa `std::fs` en ambos extremos) |
| `resolve_filename_conflict` | `fs/transfer/conflict.rs` | ✅ Local. Necesita equivalente SSH |
| `TransferWorker::run` con escaneo BFS, cycle-detection, pause/cancel, hash verify, symlink-as-symlink, scan events | `fs/transfer/worker.rs` | ✅ Pero **solo local** (todo el I/O pasa por `std::fs`) |
| UI de progreso / queue / log / report HTML+CSV | `app/state/transfer_state.rs` + `fs/transfer/report.rs` | ✅ Reutilizable |
| `progress_rx` consumiendo `ProgressUpdate` | `app/app/background.rs` | ⚠️ Paralelo al nuevo motor; ambos coexisten |

### 1.2 Lo que FALTA / está ROTO

#### A. Variantes de endpoint (el agujero más grande)
| Variante | Estado actual |
|---|---|
| `local → local` | ✅ Usa `TransferEngine` (en `copy.rs`, `move.rs`, `delete.rs`) |
| `local → SSH` | ❌ Usa el modal viejo `spawn_copy_move_task` (operador `?` en `copy.rs:38-56` y `move.rs:122-145`) |
| `SSH → local` | ❌ Igual, modal viejo |
| `SSH → SSH mismo servidor` | ❌ Modal viejo (caso especial: usa `sftp.rename` para move) |
| `SSH → SSH servidores distintos` | ❌ Modal viejo, sin preservar metadata ni verificar hash, sin buffer grande, sin pause/cancel, sin report |

El `TransferJob` actual **no modela el origen ni el destino**; solo guarda `PathBuf`. No hay forma de saber si un path vive en un panel local o en una conexión SFTP.

#### B. Acciones que NO pasan por el motor
| Acción | Hoy | Debe pasar por |
|---|---|---|
| `Copy` | Bifurca: local usa motor, SSH usa modal viejo | Motor siempre |
| `Move` | Bifurca: local usa motor, SSH usa modal viejo | Motor siempre |
| `Delete` (local) | Motor ✅ | Motor |
| `Delete` (SSH) | Modal viejo `spawn_ssh_delete_task` | Motor |
| `Rename` (local) | `std::fs::rename` directo en `actions/fs_ops/rename.rs:43` | Motor (operación `Rename`) |
| `Rename` (SSH) | No implementado (cae al popup de error o falla silenciosa) | Motor |
| `WipeFile` | `spawn_wipe_task` (`fs/ops_worker/wipe.rs`) | Motor (modo seguro de `Delete`) |
| `CreateLink` (hard/sym) | `fs/link.rs` con `std::fs` directo en `actions/fs_ops/link.rs` → popup `CreateLinkPrompt` | Motor (operación `CreateLink`) |
| `MkDir` | `fs/mkdir.rs` directo | Se mantiene fuera (no necesita progreso ni queue) — documentar excepción |

#### C. Symlinks, permisos, ACL, streams, xattr — huecos
- **Symlinks remotos:** el worker actual solo "recrea" el symlink leyendo `read_link` y llamando `symlink`/`symlink_file` de `std`. No funciona contra SFTP.
- **xattr Unix** (`user.*`, `system.posix_acl_*`): **no implementado**. Hay que añadir lectura con `xattr` crate o `listxattr/getxattr/setxattr` libc, y replicar al destino.
- **Permisos Unix al mover:** `Move` borra origen solo con `std::fs::remove_file` (no preserva timestamps del directorio padre movido, no recursivo profundo en `Move` cuando hay errores parciales).
- **Permisos de archivos creados por SFTP** no se setean al `sftp.create`; `sftp.fsetstat` permite setear `permissions` después.
- **Recursive chmod antes de borrar** (`make_writable_helper`) solo existe en local.

#### D. Estado de UI duplicado
- `state.progress_rx` + `state.active_bg_op` (sistema viejo) y `state.transfer` (motor nuevo) **conviven**. Cuando entra por SSH, el viejo maneja progreso; cuando entra local, el nuevo maneja. Esto es exactamente la inconsistencia que el refactor debe eliminar.

#### E. Tests
- Tests del motor: `worker.rs` tiene 2 tests (move de árbol, symlink circular). No hay tests para variantes SSH, ni para delete, ni para rename, ni para copy con preserve_xattr. Hay que ampliar.

---

## 2. Diseño propuesto

### 2.1 Concepto central: `TransferEndpoint`

Introducir un enum que modela **de dónde se lee y a dónde se escribe** cada archivo. Es la pieza que conecta el motor (agnóstico) con los detalles de cada medio (local vs SFTP).

```rust
// fs/transfer/endpoint.rs (NUEVO)
#[derive(Clone)]
pub enum TransferEndpoint {
    Local,
    Ssh(SharedSshClient),
}

impl TransferEndpoint {
    /// Aísla un PathBuf "lógico" + el endpoint en un handle opaco
    /// que el worker puede pasar a las funciones de I/O.
    pub fn open_reader(&self, path: &Path) -> Result<Box<dyn AsyncRead + Send + Unpin>>;
    pub fn open_writer(&self, path: &Path, overwrite: bool) -> Result<Box<dyn AsyncWrite + Send + Unpin>>;
    pub fn stat(&self, path: &Path) -> Result<StatInfo>;   // size, is_dir, is_symlink, mtime, atime, mode, uid, gid
    pub fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>>;
    pub fn read_link(&self, path: &Path) -> Result<PathBuf>;
    pub fn create_symlink(&self, target: &Path, link: &Path, is_dir: bool) -> Result<()>;
    pub fn mkdir_all(&self, path: &Path) -> Result<()>;
    pub fn rename(&self, from: &Path, to: &Path) -> Result<()>;  // mismo filesystem
    pub fn remove_file(&self, path: &Path) -> Result<()>;
    pub fn remove_dir(&self, path: &Path) -> Result<()>;
    pub fn remove_dir_all(&self, path: &Path) -> Result<()>;
    pub fn set_permissions(&self, path: &Path, mode: u32) -> Result<()>;
    pub fn set_timestamps(&self, path: &Path, atime: SystemTime, mtime: SystemTime) -> Result<()>;
    pub fn set_owner(&self, path: &Path, uid: u32, gid: u32) -> Result<()>;     // solo Unix
    pub fn get_xattr(&self, path: &Path, name: &str) -> Result<Option<Vec<u8>>>; // Unix
    pub fn set_xattr(&self, path: &Path, name: &str, value: &[u8]) -> Result<()>; // Unix
    pub fn list_xattrs(&self, path: &Path) -> Result<Vec<String>>;                 // Unix
    pub fn get_acl(&self, path: &Path) -> Result<AclInfo>;                        // Windows + Unix POSIX ACL
    pub fn set_acl(&self, path: &Path, acl: &AclInfo) -> Result<()>;
    pub fn list_ads(&self, path: &Path) -> Result<Vec<String>>;                   // Windows
    pub fn copy_ads(&self, src: &Path, dst: &Path) -> Result<()>;                 // Windows
    pub fn same_filesystem_as(&self, other: &Self) -> bool;                       // habilita move atómico
}
```

**Justificación de `Box<dyn>`:** los endpoints son heterogéneos (Local tiene un set de métodos POSIX, Ssh tiene un set distinto limitado por SFTP v3/v6), y el motor los trata polimórficamente. El costo de dyn en un task de copia es despreciable comparado con el I/O real. La alternativa con genéricos forzaría monomorfizar todo el motor y haría muy difícil añadir un tercer endpoint (FUSE, WebDAV, etc.) en el futuro.

### 2.2 Adaptador SFTP

`SharedSshClient` ya tiene la base. Ampliarlo (`fs/ssh.rs`) con todo lo de arriba. Para las operaciones no soportadas por SFTP v3 (e.g. `set_owner`, `xattr`), recurrir a `session.exec("chmod ...")` / `("chown ...")` / `("setfattr -n user.x -v y ...")` o, cuando no hay shell, documentar el límite y degradar gracefully (registrar warning, no fallar la operación).

### 2.3 Modelo de datos del job

```rust
// fs/transfer/job.rs (MODIFICADO)
pub struct TransferJob {
    pub id: Uuid,
    pub operation: TransferOperation,    // ahora: Copy | Move | Delete | Rename | CreateLink | Wipe
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,            // para Rename: es el path destino; para Delete/CreateLink: vacío
    pub src_endpoint: TransferEndpoint,
    pub dst_endpoint: TransferEndpoint,   // para Delete: == src_endpoint; para CreateLink: == src_endpoint
    pub options: TransferOptions,
    pub status, results, progress, log_lines, is_paused, is_cancelled, skip_file_flag, active_conflict
}

pub enum TransferOperation {
    Copy,
    Move,
    Delete,        // incluye wipe si options.wipe_pass > 0
    Rename,        // sources.len() == 1; destination es el path final
    CreateLink {   // sources.len() == 1; destination es el link nuevo
        kind: LinkKind,    // Symbolic | Hard
    },
}
```

### 2.4 Pipeline unificado

Reemplazar `copy_file_pipelined` por una versión que reciba `TransferEndpoint` para cada extremo:

```rust
// fs/transfer/pipeline.rs (REESCRITO)
pub async fn copy_file_pipelined(
    src_endpoint: &TransferEndpoint,
    src: &Path,
    dst_endpoint: &TransferEndpoint,
    dst: &Path,
    options: &TransferOptions,
    event_tx: &UnboundedSender<TransferEvent>,
    job_id: Uuid,
    is_paused: Arc<AtomicBool>,
    is_cancelled: Arc<AtomicBool>,
    bytes_transferred_acc: Arc<AtomicU64>,
) -> Result<(Option<String>, Option<String>), anyhow::Error>;
```

Mantiene el patrón reader/writer en `spawn_blocking`, con backpressure vía `mpsc::channel(4)`. El reader usa `src_endpoint.open_reader(src)`, el writer `dst_endpoint.open_writer(dst)`. Los hashes (CRC32/MD5/SHA1/SHA256/BLAKE3) se mantienen streaming en cada lado independientemente.

**Optimización de mismo-filesystem:** para `Move` donde `src_endpoint == dst_endpoint && src_endpoint.same_filesystem_as(dst_endpoint) && !cross_device`, hacer `endpoint.rename(src, dst)` atómico en vez de copy+delete. El worker detecta esto y se salta la pipeline.

### 2.5 Worker unificado

`TransferWorker::run` se reescribe como un dispatcher por operación. El escaneo (fase 1) y la fase de I/O (fase 2) son endpoint-agnósticos, pero cada acción concreta delega al endpoint:

```
match self.operation {
    Copy    => ejecutar copy_phase,
    Move    => { si same_endpoint_and_fs: ejecutar move_atómico; si no: copy_phase + delete_phase }
    Delete  => ejecutar delete_phase (con wipe si options.wipe_passes > 0)
    Rename  => rename_atómico (un solo path; errores con admin retry)
    CreateLink { kind } => crear hard o symlink (preservar target, modo)
}
```

`preserve_metadata` se vuelve polimórfico: recibe `(src_endpoint, src, dst_endpoint, dst, &options)` y aplica timestamps, permisos, atributos Windows, ACL, xattr, ADS según `options` y según lo que cada endpoint soporte.

### 2.6 Escaneo de árbol (fase 1)

Hoy el escaneo es `std::fs::read_dir` + `canonicalize` para cycle detection. Convertirlo a `src_endpoint.read_dir` + `src_endpoint.stat` con un set de visitados por `inode` (no por path canónico, porque SFTP no siempre expone inodes estables — usar `(dev, ino)` cuando esté disponible, fallback a `path` para SSH).

### 2.7 Manejo de symlinks

| Modo | Comportamiento |
|---|---|
| `skip_symlinks = true` | Symlinks no se transfieren (warning al usuario) |
| `follow_symlinks = true` | Se sigue el symlink, se copia el **target** como archivo regular |
| `skip = false, follow = false` (default) | Se **recrea** el symlink en destino con el mismo target (preservando relativo/absoluto y tipo: file vs dir en Windows) |

Para SFTP: `sftp.symlink(target, link_path)` y, en Windows sobre SSH, hay que ejecutar `cmd /c mklink` (no hay primitiva SFTP portable). Documentar la limitación y degradar a `sftp.symlink` cuando el server lo acepte.

### 2.8 Permisos, ACL, xattr, ADS

Implementar en `fs/transfer/metadata.rs` (ya existente) las versiones endpoint-aware:

- `preserve_timestamps` → `endpoint.set_timestamps(dst, atime, mtime)`. En SFTP usar `sftp.setstat` con `FileStat` (mtime, atime). Si no es soportado por el server, warning y skip.
- `preserve_attributes` → Unix: `endpoint.set_permissions(dst, mode)`. Windows: `SetFileAttributesW`.
- `preserve_acl`:
  - Unix: usar `acl` crate (o `getfacl`/`setfacl` por SSH) para POSIX.1e ACL. Fallback a `chmod` si no hay ACL extendida.
  - Windows: `GetNamedSecurityInfoW` + `SetFileSecurityW` (ya implementado, pero en `metadata.rs` hay que pasarlo al endpoint para que use la ruta correcta).
  - SFTP: el server no expone ACL portable; usar `sftp.fsetstat` con `FileStat` para Unix mode; degradar gracefully si no hay.
- `preserve_streams` (ADS Windows): ya implementado. El endpoint SFTP-Server-Windows no soporta ADS en general; warning.
- **xattr Unix** (NUEVO): añadir `xattr` crate a `Cargo.toml` (`[target.'cfg(unix)'.dependencies]`). `endpoint.get_xattr/list_xattrs/set_xattr` lo usa. Para SFTP-Unix: si el server expone `xattr` vía SFTP v6+, usar `fsetstat` extendido; si no, ejecutar `setfattr -n name -v value file` por `session.exec`.

### 2.9 Renombrar a través del motor

`Rename` es un caso particular: una sola entrada, mismo endpoint en origen y destino (no tiene sentido renombrar entre dos servidores). El worker hace `src_endpoint.rename(src, dst)`. Si falla por permisos, usar el mismo flujo de `elevated_helper` que ya existe en `fs/elevated_helper.rs`. Si el destino existe, aplicar la resolución de conflictos (renombrar automáticamente) igual que Copy.

### 2.10 Crear enlace a través del motor

`CreateLink { kind }`: una sola entrada fuente, mismo endpoint. `kind`:
- `Symbolic` → `endpoint.create_symlink(target, link_path, target_is_dir)`. Validar que el target sea accesible.
- `Hard` → `endpoint.create_hardlink(src, link_path)`. Solo mismo filesystem (mismo `dev`); si no, error claro.

El popup `CreateLinkPrompt` se mantiene; lo que cambia es que al confirmar se encola un `TransferJob::CreateLink` y el popup de progreso es el mismo que para copy.

### 2.11 Eliminar modales viejos (limpieza final)

Cuando todo lo anterior esté en su lugar y los call sites migrados:

| Archivo | Acción |
|---|---|
| `fs/ops_worker/mod.rs` | Borrar las re-exports de `spawn_copy_move_task` y `spawn_ssh_delete_task` |
| `fs/ops_worker/copy_move.rs` | **Borrar** (~380 líneas) |
| `fs/ops_worker/delete.rs` | **Borrar** (~60 líneas) |
| `fs/ops_worker/helper.rs` (la parte de `delete_recursive`) | Mover lo que quede útil a `endpoint.rs`; borrar el resto |
| `fs/ops_worker/wipe.rs` | Refactorizar para que también delegue al motor (`TransferOperation::Delete` con `wipe_passes > 0`), o **borrar** si decidimos que wipe es solo `Delete` con opción |
| `app/actions/fs_ops/copy.rs` | Quitar el `if is_ssh { ... } else { ... }`; siempre ir al motor |
| `app/actions/fs_ops/move.rs` | Igual |
| `app/actions/fs_ops/delete.rs` | Quitar el `if let Some(client) = &active_panel.ssh_conn`; siempre ir al motor |
| `app/actions/fs_ops/rename.rs` | Reescribir `commit` para encolar un `TransferJob::Rename` |
| `app/actions/fs_ops/wipe.rs` | Reescribir para encolar `Delete` con `wipe_passes = 3` (configurable) |
| `app/actions/fs_ops/link.rs` | Reescribir `commit` para encolar `CreateLink` |
| `app/input_popup/copy.rs` (líneas 286-389) | Quitar la rama `if is_ssh`; siempre usar motor |
| `app/input_popup/rename_move.rs` (líneas 285-298) | Reescribir para encolar motor |
| `app/input_popup/delete.rs` (líneas 30-89) | Quitar la rama SSH; siempre motor |
| `app/app/background.rs` | Cuando se detecte que ningún `progress_rx` queda en uso, **borrar** el bloque 1 entero (líneas 21-117) y `state.active_bg_op` |
| `app/state/types.rs` (`BackgroundOpContext`) | **Borrar** el enum |
| `app/state/mod.rs` | Quitar `progress_rx`, `active_bg_op` |

---

## 3. Plan de implementación por fases

> **Regla general por fase:** terminar cada fase con `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` pasando, y al menos un test nuevo que cubra el caso. **No acumular fases sin verificar.**

### Fase 0 — Higiene previa (1 commit)
- Correr `cargo clippy --all-targets -- -D warnings` y limpiar lo que aparezca hoy. No tocar el sistema de transferencia todavía. Sirve de línea base.
- `git commit` snapshot.

### Fase 1 — Introducir `TransferEndpoint` y adaptador SFTP (sin tocar el motor)
1. Crear `fs/transfer/endpoint.rs` con la enum `TransferEndpoint` y su trait method set completo.
2. Implementar `Local` totalmente (envuelve `std::fs` y las APIs Windows/Unix).
3. Implementar `Ssh(client)` que envuelve `SharedSshClient`; para lo que SFTP no soporte, usar `session.exec` (chmod/chown/setfattr).
4. Añadir `xattr` crate a `Cargo.toml` (`[target.'cfg(unix)'.dependencies] xattr = "1"`).
5. Tests unitarios del endpoint `Local` (los que sean puros de path, sin I/O real).
6. Verificar: `cargo test` y `cargo clippy` limpios.
7. `git commit`.

### Fase 2 — Refactor de `preserve_metadata` a endpoint-aware
1. Reescribir `preserve_metadata` para que reciba `(src_endpoint, src, dst_endpoint, dst, &options)`.
2. Mantener compatibilidad: el path local sigue funcionando porque `Local` envuelve las mismas APIs.
3. Tests: añadir `test_preserve_metadata_local_keeps_timestamps` y `test_preserve_metadata_local_keeps_mode`.
4. `git commit`.

### Fase 3 — Refactor de `copy_file_pipelined` a endpoint-aware
1. Reescribir firma como en §2.4.
2. Mantener toda la lógica existente: pipeline reader/writer, hash streaming, bandwidth throttling, direct I/O con fallback. **Importante:** `DirectIO` solo aplica a `Local`; para `Ssh` se ignora (SFTP no tiene direct I/O).
3. `AlignedBuffer` solo se usa en `Local`.
4. Tests: añadir test que use `tempfile` con un endpoint `Local` y verifique que el archivo resultante tiene el mismo contenido y, si `verify_after_copy = true`, que los hashes coinciden.
5. `git commit`.

### Fase 4 — Refactor del `TransferWorker` a endpoint-aware
1. `TransferJob` gana `src_endpoint` y `dst_endpoint` (con default `Local` en el constructor para no romper).
2. `TransferWorker::run` reescrito como dispatcher (§2.5).
3. El escaneo usa `endpoint.read_dir` y `endpoint.stat`; cycle-detection por `(dev, ino)` o path.
4. `preserve_metadata` polimórfico.
5. Las variantes SSH ya funcionan **sin que aún se migren los call sites** (los call sites actuales crean jobs `Local`; los SSH siguen yendo por el modal viejo).
6. Tests:
   - `test_worker_copy_local_to_local` (ya hay implícito; hacerlo explícito)
   - `test_worker_delete_local` (recicla `test_worker_move_directory_tree` pero con `TransferOperation::Delete`)
   - `test_worker_rename_local`
   - `test_worker_create_symlink_local`
   - `test_worker_copy_with_preserve_timestamps_and_mode`
7. `git commit`.

### Fase 5 — Migrar los call sites de Copy y Move
1. `app/actions/fs_ops/copy.rs`: borrar la rama `is_ssh`. Ahora siempre va al motor. El motor detecta `src_endpoint`/`dst_endpoint` mirando `state.get_active_panel().ssh_conn` y `state.get_passive_panel().ssh_conn`.
2. `app/actions/fs_ops/move.rs`: igual.
3. `app/input_popup/copy.rs`: borrar la rama `is_ssh` en el bloque `Enter`.
4. `app/input_popup/rename_move.rs`: reescribir para encolar `TransferJob::Move` siempre.
5. Verificar manualmente el flujo: copiar local→local, local→SSH, SSH→local, SSH→SSH mismo server, SSH→SSH cross-server.
6. `git commit`.

### Fase 6 — Migrar Delete
1. `app/actions/fs_ops/delete.rs`: borrar rama SSH; encolar `TransferJob::Delete` siempre.
2. `app/input_popup/delete.rs` (popup `ConfirmDelete`): borrar rama SSH; encolar siempre.
3. Verificar delete local + delete SSH (incluyendo el caso de recycle bin, que solo aplica a `Local`).
4. `git commit`.

### Fase 7 — Rename a través del motor
1. Añadir `TransferOperation::Rename`.
2. `app/actions/fs_ops/rename.rs` → `commit` encola `TransferJob::Rename`.
3. Popup `RenamePrompt` se mantiene; tras Enter, encolar job.
4. Tests: `test_rename_local_basic`, `test_rename_local_over_existing_renames` (conflicto).
5. `git commit`.

### Fase 8 — CreateLink a través del motor
1. Añadir `TransferOperation::CreateLink { kind }`.
2. `app/actions/fs_ops/link.rs` → encolar `TransferJob::CreateLink`.
3. Verificar symlink local y hardlink local; SSH se documenta como "soportado si el server lo permite".
4. Tests: `test_create_symlink_local`, `test_create_hardlink_local`.
5. `git commit`.

### Fase 9 — Wipe a través del motor
1. Decisión: ¿`TransferOperation::Wipe` separado o `Delete` con `wipe_passes > 0`? **Recomiendo la segunda** (más simple, mismo flujo de UI, un solo punto de cambio).
2. Añadir `wipe_passes: u8` a `TransferOptions` (default 0 = no wipe).
3. Cuando `wipe_passes > 0`, antes de `remove_file`, sobrescribir el contenido con `0x00`, `0xFF`, `0x00` y luego borrar.
4. Migrar `app/actions/fs_ops/wipe.rs` y `app/input_popup/delete.rs` (popup `WipeConfirm`) para que encole `Delete` con `wipe_passes = 3`.
5. Borrar `fs/ops_worker/wipe.rs`.
6. `git commit`.

### Fase 10 — Limpieza final (lo más delicado)
> **Hacer todo en un commit atómico, no por mitades.** Antes de empezar, congelar la versión.

1. Borrar `fs/ops_worker/{copy_move,delete}.rs` y el subdirectorio queda con `compress.rs`, `extract.rs`, `helper.rs` (mover a endpoint si hay funciones útiles) y `wipe.rs` (si quedó algo).
2. En `fs/ops_worker/mod.rs`: quitar re-exports de `spawn_copy_move_task`, `spawn_ssh_delete_task`, `spawn_wipe_task`. Mantener `spawn_compress_task` y `spawn_extract_task` (compress/extract son casos especiales que el motor de transferencia todavía no cubre; ver §5.2).
3. En `app/app/background.rs`: borrar el bloque 1 (líneas 21-117) y la rama que maneja `progress_rx`. El motor de transferencia ya tiene su propio event loop.
4. En `app/state/types.rs`: borrar `BackgroundOpContext`.
5. En `app/state/mod.rs`: borrar `progress_rx`, `active_bg_op`; quitar su inicialización en `AppState::new`.
6. En `fs/mod.rs`: quitar `pub use ops_worker::{ProgressUpdate, spawn_compress_task, spawn_copy_move_task, spawn_extract_task, spawn_ssh_delete_task, spawn_wipe_task};` y reemplazar por `pub use ops_worker::{spawn_compress_task, spawn_extract_task};` y `pub use transfer::endpoint::TransferEndpoint;` y `pub use transfer::pipeline::copy_file_pipelined;`.
7. Buscar con `rg "progress_rx|active_bg_op|BackgroundOpContext|spawn_copy_move_task|spawn_ssh_delete_task|spawn_wipe_task"` y eliminar toda referencia.
8. `cargo check && cargo clippy --all-targets -- -D warnings && cargo test`.
9. `git commit`. **Tag `v0.8.0-refactor-unified-transfer`.**

### Fase 11 — Documentación y changelog
1. Cargar skill `changelog-helper` y actualizar `CHANGELOG.md`.
2. Cargar skill `documentation-writer` y actualizar `docs/transfer.md` (si existe) o crearlo.
3. Cargar skill `localize-helper` si se añadieron strings de UI.
4. `git commit`.

---

## 4. Pruebas nuevas obligatorias (resumen)

| Test | Cubre |
|---|---|
| `endpoint_local_roundtrip` | `Local::open_reader`+`open_writer`+`stat` |
| `endpoint_local_symlink_create_and_read` | `create_symlink` + `read_link` |
| `worker_copy_local_to_local_preserves_mode` | `preserve_attributes` end-to-end |
| `worker_copy_local_to_local_preserves_timestamps` | `preserve_timestamps` end-to-end |
| `worker_copy_local_to_local_preserves_xattr` | xattr Unix (gated `#[cfg(unix)]`) |
| `worker_copy_local_to_local_preserves_ads` | ADS Windows (gated `#[cfg(windows)]`) |
| `worker_copy_with_skip_symlinks` | modo skip |
| `worker_copy_with_follow_symlinks` | modo follow (no recrea, copia el target) |
| `worker_copy_recreates_relative_symlink` | modo default (target relativo preservado) |
| `worker_delete_local_recursive` | delete con subdirs |
| `worker_delete_local_wipe_overwrites_bytes` | `wipe_passes = 3` deja bytes en 0x00 o 0xFF |
| `worker_rename_local` | rename básico |
| `worker_rename_local_renames_on_conflict` | `conflict_resolution = "rename"` aplica `resolve_filename_conflict` |
| `worker_create_symlink_local` | create_link Symbolic |
| `worker_create_hardlink_local` | create_link Hard |
| `worker_copy_resume_from_interrupted` | simular cancelación a la mitad; relanzar; verifica que termina sin duplicar |
| `engine_serializes_two_jobs` | la cola procesa 1 a la vez (regresión) |
| `engine_pause_resume` | is_paused se respeta |
| `engine_cancel_terminates_cleanly` | is_cancelled no deja fugas |
| `pipeline_bandwidth_throttle_caps_rate` | con `limit_bandwidth_rate` no excede N bytes/s |
| `pipeline_verify_hash_mismatch_records_failure` | `verify_after_copy` con datos distintos en destino |

**Criterio de aceptación:** todos los tests pasan localmente. CI verde.

---

## 5. Decisiones y excepciones explícitas

### 5.1 No tocamos ahora (fuera de alcance)
- **Compress/Extract:** siguen por `fs/ops_worker/{compress,extract}.rs` porque su modelo (entrada/salida son streams sobre archivos comprimidos, no árboles) es distinto. La skill que se carga desde aquí para extraer, eventualmente, también debería pasar por un job del motor, pero tiene su propio lifecycle (un solo archivo o un solo tarball). Se deja como ticket aparte.
- **Mkdir:** no necesita cola, ni progreso, ni reporte. Se mantiene fuera del motor. Está bien así.
- **Apply command / describe / view / edit / wipe de archivos individuales desde menú contextual:** sin cambios (no son transferencias).

### 5.2 Degradaciones explícitas (decisión confirmada: SSH = solo SFTP nativo)
- **Direct I/O** solo aplica a `Local`. Para `Ssh` se ignora silenciosamente y se registra un warning al log.
- **xattr sobre SSH:** SFTP v3 no expone xattr. **No se usa `session.exec` como fallback.** Se loguea un warning y se omite.
- **ADS sobre SSH:** SFTP no soporta ADS. **No se usa `session.exec` como fallback.** Se loguea un warning y se omite.
- **ACL POSIX sobre SSH:** SFTP solo expone permisos Unix via `fsetstat`. **No se usa `session.exec` para getfacl/setfacl.** Se preservan solo los permisos Unix básicos (modo), no las ACL extendidas; warning.
- **Atributos Windows sobre SSH:** no aplica (SFTP-Unix no tiene esos atributos). Sin cambios.
- **Hardlinks cruzando filesystems:** error claro, no se intenta. El popup de UI sugiere convertirlo a symlink.

**Implicancia:** el crate `xattr` solo se usa cuando `endpoint == Local && target_os = unix`. No se añade dependencia nueva para SSH.

### 5.3 Compatibilidad hacia atrás
- `Settings.transfer_*` y la estructura de `TransferOptions` se mantienen; solo se **amplían** con campos nuevos (`wipe_passes`, etc.). El `config.toml` viejo sigue cargando.
- `PopupType::CopyProgress` (que ya existe para el modal viejo) se va a eliminar junto con el modal viejo en Fase 10; el motor ya tiene su propio popup `TransferPanel`.

---

## 6. Riesgos y mitigaciones

| Riesgo | Probabilidad | Mitigación |
|---|---|---|
| `sftp.fsetstat` con campos no soportados hace que el server rechace la operación | Media | Probar con campos opcionales uno a uno; envolver en `try { ... } catch { warn }` |
| SFTP renombrar entre directorios en el mismo server **no** es atómico si cruza filesystem | Media | Detectar `same_filesystem_as` y degradar a copy+delete |
| SFTP lock contention con el SFTP que está leyendo el árbol para listar | Media | El SFTP lock ya se libera entre operaciones; verificar en `SharedSshClient` que `delete_recursive` no lo mantiene durante la recursión (ya está corregido en el código actual) |
| El refactor rompe los keybindings porque la acción `Copy` cambia de path de ejecución | Baja | `Action::Copy` se mantiene igual; lo que cambia es el handler interno. Sin impacto en keymaps |
| Regresión en pausa/cancel | Media | Mantener los 2 tests existentes (`test_worker_move_directory_tree`, `test_worker_scan_does_not_loop_on_circular_symlink`) y añadir `test_pause_resume` y `test_cancel_terminates_cleanly` |
| Performance del polimorfismo `Box<dyn>` | Baja | Medir antes/después con un archivo de 1 GB. Si >5% peor, cambiar a `enum` con match en sitios críticos (es lo que hacen la mayoría de file managers) |
| Olvidar migrar algún call site y dejar `progress_rx` colgando | Media | Fase 10 hace `rg` global antes de borrar. Si queda alguna referencia, `cargo check` falla y se corrige |

---

## 7. Estimación de tamaño

| Fase | Líneas estimadas (cambio) | Archivos tocados |
|---|---|---|
| 0 | 0 | 0 |
| 1 | +600 / -0 | 1 nuevo, 1 modificado |
| 2 | +200 / -100 | 1 |
| 3 | +300 / -150 | 1 |
| 4 | +500 / -400 | 1 |
| 5 | +50 / -100 | 4 |
| 6 | +30 / -60 | 2 |
| 7 | +150 / -50 | 1 + 1 test |
| 8 | +120 / -30 | 1 + 1 test |
| 9 | +80 / -100 | 1 + borrar 1 |
| 10 | +20 / -800 | ~10 (borrados y limpieza) |
| 11 | +200 | 3 docs |
| **Total** | **+2250 / -1790** | ~25 archivos |

Neto: +460 líneas (el motor crece, los modales viejos se van). Aproximadamente 8-12 horas de trabajo concentrado para alguien con el contexto.

---

## 8. Verificación final antes de cerrar

- [ ] `cargo check --all-targets` sin warnings
- [ ] `cargo clippy --all-targets --all-features --locked -- -D warnings` limpio
- [ ] `cargo test --all` pasa (incluyendo los 20+ tests nuevos)
- [ ] `cargo fmt --all -- --check` limpio
- [ ] Smoke test manual:
  - [ ] Copy local → local
  - [ ] Copy local → SSH
  - [ ] Copy SSH → local
  - [ ] Copy SSH → SSH mismo server
  - [ ] Copy SSH → SSH cross-server (en VM)
  - [ ] Move local → local
  - [ ] Move SSH → SSH mismo server (atómico)
  - [ ] Move SSH → SSH cross-server
  - [ ] Delete local con recycle bin
  - [ ] Delete local sin recycle bin
  - [ ] Delete SSH
  - [ ] Rename local (sin y con conflicto)
  - [ ] Create symlink local
  - [ ] Create hardlink local
  - [ ] Wipe local (3 pasadas)
  - [ ] Symlink preservado (modo default)
  - [ ] Symlink skipped (skip_symlinks)
  - [ ] Symlink followed (follow_symlinks)
  - [ ] Permissions preservados (Unix + Windows)
  - [ ] ACL preservado (Windows + Unix POSIX)
  - [ ] ADS preservado (Windows)
  - [ ] xattr preservado (Unix)
  - [ ] Pausa y reanudación
  - [ ] Cancelación limpia
  - [ ] Verificación por hash OK
  - [ ] Verificación por hash falla → registra failure
  - [ ] Reporte HTML generado
  - [ ] Reporte CSV generado
- [ ] `CHANGELOG.md` actualizado (skill `changelog-helper`)
- [ ] Tag de versión nuevo (`v0.8.0-refactor-unified-transfer`)
- [ ] `rg "spawn_copy_move_task|spawn_ssh_delete_task|spawn_wipe_task|progress_rx|active_bg_op|BackgroundOpContext"` devuelve 0 hits

---

## 9. Resumen ejecutivo

**Lo que hay que hacer, en una línea:** introducir un `TransferEndpoint` polimórfico (Local y Ssh), reescribir el motor (`pipeline`, `worker`, `metadata`, `job`) para que sea endpoint-agnóstico, ampliar `TransferOperation` con `Rename`, `CreateLink` y `Wipe` (este último como opción de `Delete`), migrar los 6 call sites de copy/move/delete/rename/wipe/create_link para que **siempre** pasen por el motor, y borrar los archivos del sistema viejo (`fs/ops_worker/{copy_move,delete,wipe}.rs` y el campo `progress_rx`/`active_bg_op` del estado).

**Lo que se gana:** un único motor, una única cola, una única UI de progreso, un único sistema de reportes, soporte completo para symlinks/permisos/ACL/ADS/xattr en **todas** las combinaciones de paneles, y aproximadamente 800 líneas de código viejo borradas.

**Lo que se arriesga:** regresiones en SSH si la API SFTP no se comporta uniforme entre servers. Mitigado con tests por capability y degradaciones explícitas.
