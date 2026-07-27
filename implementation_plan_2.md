# Plan: Migración final del sistema de transferencia

> **Objetivo:** terminar la unificación del motor de transferencia.
> Cuatro sub-proyectos en orden: Compress/Extract, elevación
> (retry-as-admin), Direct I/O confirmation, xattr/ACL/ADS
> confirmation.

> ⚠️ **Estado del plan:** ✅ Aprobado por el usuario (2026-07-27).
> En ejecución. Commits van a `refactor-plugins`.
>
> 🔴 **PENDIENTE — SFTP v6 hardlinks:** Diferido a futuro. No se
> implementa en esta fase. Razones y costos estimados en §"SFTP
> v6 hardlinks (deferido)" al final del documento. Cuando se
> levante, va como fase propia (~1-2 días, +150 líneas, requiere
> server v6 real para testear).

---

## Sub-proyecto 1 — Compress/Extract al motor unificado (6-8 días)

### 1.1 Estado actual

`fs/ops_worker/compress.rs` y `fs/ops_worker/extract.rs` son
los dos únicos paths que **no** pasan por el motor nuevo.
Comparten `state.progress_rx` y el popup `CopyProgress` viejo
con todos los file ops. Esa duplicación es exactamente lo que
queremos eliminar.

Las crates de formato ya están todas en `Cargo.toml`:

| Formato | Crate | Notas |
|---|---|---|
| ZIP | `zip = "8.6.0"` con `deflate` y `time` | El más usado, soporta timestamps ZIP. |
| TAR | `tar = "0.4.46"` | Sin compresión propia, se combina con `flate2` para `.tar.gz`. |
| GZIP | `flate2 = "1.1.9"` con `rust_backend` | Solo para `.gz` puro (un solo archivo). |
| 7Z | `sevenz-rust = "0.6.1"` | Lector y escritor. |

**No agregamos crates nuevas.** Sí hay que activar un feature
de `zip` (`deflate` ya está; si quisiéramos AES-password hay
que añadir `aes-crypto` — pero **no lo agregamos** en esta
fase, ver §1.6).

### 1.2 Diseño

Dos nuevas variantes en `TransferOperation`:

```rust
// fs/transfer/job.rs
pub enum ArchiveFormat {
    Zip,
    TarGz,  // .tar.gz
    SevenZ,
}

pub enum TransferOperation {
    // ... las que ya están ...
    Compress { format: ArchiveFormat, level: u8 },
    Extract { format: ArchiveFormat },
}
```

- `Compress.sources` = archivos/carpetas a incluir.
  `destination` = path del archivo de salida (ej. `backup.zip`).
- `Extract.sources` = un único archivo (el archive). El motor
  valida `sources.len() == 1`. `destination` = directorio
  destino.

Las dos son polimórficas vía `TransferEndpoint` igual que
Copy/Move:

- Compress local→SSH: lee de disco, escribe el archive al server.
- Compress SSH→local: lee del server, escribe el archive local.
- Compress SSH→SSH: lee de un server, escribe al mismo u otro server.

### 1.3 Pipeline nuevo

`fs/transfer/pipeline.rs` ya tiene `copy_file_pipelined`.
Agregamos dos funciones nuevas al mismo módulo, **sin**
reutilizar el byte-streaming existente (compress/extract
tienen estructura de "muchos readers, un writer" y "un
reader, muchos writers" respectivamente).

```rust
// fs/transfer/pipeline.rs (firmas, no implementación)

/// Stream a set of source files/folders into a single archive.
/// The format drives the encoder. `level` is the compression
/// level (0-9; 0 means "store only" for zip, ignored for tar.gz).
/// Returns Ok with the archive size in bytes.
pub async fn compress_pipeline(
    src_endpoint: &TransferEndpoint,
    sources: Vec<PathBuf>,
    dst_endpoint: &TransferEndpoint,
    archive: &Path,
    format: ArchiveFormat,
    level: u8,
    event_tx: &mpsc::UnboundedSender<TransferEvent>,
    job_id: Uuid,
    is_paused: Arc<AtomicBool>,
    is_cancelled: Arc<AtomicBool>,
    bytes_transferred_acc: Arc<AtomicU64>,
) -> Result<u64, anyhow::Error>;

/// Stream an archive into a destination directory.
/// Returns Ok with the number of entries extracted.
pub async fn extract_pipeline(
    src_endpoint: &TransferEndpoint,
    archive: &Path,
    dst_endpoint: &TransferEndpoint,
    dst_dir: &Path,
    format: ArchiveFormat,
    event_tx: &mpsc::UnboundedSender<TransferEvent>,
    job_id: Uuid,
    is_paused: Arc<AtomicBool>,
    is_cancelled: Arc<AtomicBool>,
    bytes_transferred_acc: Arc<AtomicU64>,
) -> Result<u64, anyhow::Error>;
```

Ambas respetan pause/cancel vía los `Arc<AtomicBool>` ya
existentes, igual que `copy_file_pipelined`.

### 1.4 Comportamiento por formato

**ZIP (`zip`):**

- Compress: `zip::write::FileOptions::default().compression_method(...)`.
  Recorrer `sources` recursivamente con `std::fs::read_dir` o
  `endpoint.read_dir`, por cada entry abrir el writer entry
  del zip, hacer `copy_to_end(writer)` con un `std::io::Read` del
  endpoint. El path dentro del zip es el path relativo al
  primer source (si hay uno) o al parent del archive
  (si hay varios sources, "sources/relpath").
- Extract: `zip::ZipArchive::new(reader)`, iterar entries,
  para cada uno abrir el writer del endpoint en
  `dst_dir.join(sanitize(entry.name()))`. Sanitización:
  rechazar `..`, paths absolutos, NUL bytes (mismo
  `validate_archive_entry_name` que ya existe en
  `fs/archive.rs`).

**TAR.GZ (`tar` + `flate2`):**

- Compress: `flate2::write::GzEncoder::new(writer, level)` →
  `tar::Builder::new(encoder)`. Por cada source recursivo,
  `append_dir` / `append_file` con un reader del endpoint.
- Extract: `flate2::read::GzDecoder::new(reader)` →
  `tar::Archive::new(decoder)`. Iterar entries, sanitizar,
  escribir al endpoint.

**7Z (`sevenz-rust`):**

- Compress: la API de `sevenz_rust` es bloqueante (no async).
  La llamamos dentro de `tokio::task::spawn_blocking` igual
  que la pipeline actual. Output a un `Vec<u8>` intermedio o
  streaming al writer del endpoint.
- Extract: idem, `Archive::read` con un callback que va
  escribiendo a los entries. Ya hay extract en
  `fs/archive.rs`; miramos de reusar la lógica de
  sanitización de nombres.

### 1.5 Worker dispatcher

```rust
// fs/transfer/worker.rs (en el match)
TransferOperation::Compress { format, level } => {
    self.run_compress(format, level).await
}
TransferOperation::Extract { format } => {
    self.run_extract(format).await
}
```

`run_compress` / `run_extract` llaman a las pipelines de §1.3
y devuelven `TransferResults` con un único
`FileTransferResult` (el archive entero como una "entrada
lógica") más un `SkippedFile` por cada entry que se saltó en
extract por conflicto.

### 1.6 Decisiones explícitas

- **No password / AES.** El código actual no lo soporta. No
  lo agregamos (scope creep). Si en el futuro hace falta,
  `zip` con feature `aes-crypto`.
- **Compress level default = 6** (deflate estándar). El
  popup permite 0-9, donde 0 significa "store only" en zip
  (sin compresión, útil para `.zip` rápido de archivos ya
  comprimidos como JPG/MP4).
- **Compress desde un directorio:** la convención es que el
  archive contiene `<dirname>/<files>` donde `<dirname>` es el
  nombre del directorio source. Mismo comportamiento que
  `tar -czf backup.tar.gz ./mi_carpeta`.
- **Compress desde múltiples paths:** el archive contiene
  cada path con su `basename` como root. Ejemplo: sources =
  `["/a/folder1", "/a/folder2"]` → archive contiene
  `["folder1/...", "folder2/..."]`.
- **Extract sobre SSH:** el decoder se alimenta con
  `src_endpoint.open_reader`. El writer de cada entry se
  abre con `dst_endpoint.open_writer`. No hay streaming
  directo reader→writer porque el decoder necesita procesar
  headers antes de cada entry.
- **Format detection en extract:** el popup pide el formato
  (no auto-detect) porque la UI actual ya funciona así y
  cambiarla es scope creep. El usuario selecciona el formato
  del archivo antes de hacer extract. Si el formato no
  matchea, `sevenz_rust` y `zip` devuelven error claro.

### 1.7 Migración de call sites

`app/actions/fs_ops/compress.rs` (si existe; si no, el
handler está en `app/input_popup/compress.rs`):

- En lugar de `crate::fs::spawn_compress_task(...)` con
  `state.progress_rx = Some(rx)`, encolar un
  `TransferJob::Compress` en el `TransferEngine`.
- El popup `CopyProgress` ya **no se dispara** para compress.

`app/actions/fs_ops/extract.rs`:

- Idem. Encolar `TransferJob::Extract`.

`app/app/background.rs`:

- El bloque 1 que drena `state.progress_rx` ya no recibe
  updates de compress/extract (porque esos paths ya no
  escriben ahí). El bloque se queda solo para los handlers
  viejos de Copy/Move que aún no estén migrados (en este
  punto del refactor no quedan, así que el bloque 1 entero
  se borra).

`app/state/mod.rs`:

- `state.progress_rx` se puede eliminar.

`fs/ops_worker/compress.rs` y `fs/ops_worker/extract.rs`:

- Borrar (no son referenciados por nadie más).

### 1.8 Tests nuevos

| Test | Cubre |
|---|---|
| `compress_local_to_local_zip` | Compress un dir local a un .zip local, verifica contenido. |
| `compress_local_to_local_targz` | Compress un dir local a un .tar.gz local, verifica contenido. |
| `compress_local_to_local_7z` | Compress un dir local a un .7z local, verifica contenido. |
| `compress_with_zero_level_stores_only` | Zip con level=0 es tan grande como la suma de los sources. |
| `compress_multiple_sources_flattens_names` | Sources = `[dir1, dir2]` → archive tiene `dir1/...` y `dir2/...`. |
| `extract_zip_to_local` | Round-trip: compress + extract recupera el árbol original. |
| `extract_targz_to_local` | Idem con tar.gz. |
| `extract_7z_to_local` | Idem con 7z. |
| `extract_rejects_path_traversal` | Archive malicioso con `../etc/passwd` es rechazado. |
| `pipeline_compress_cancellation_aborts` | Pre-cancel mata la operación. |
| `pipeline_extract_cancellation_aborts` | Idem. |
| `worker_compress_dispatches_correctly` | El worker dispatcher para `Compress { format: Zip }` llama a `compress_pipeline` con el formato correcto. |

Criterio: 12 tests nuevos, todos pasando, `cargo clippy -D
warnings` limpio, sin warnings de `dead_code`.

### 1.9 Fases (código)

| # | Commit | Contenido |
|---|---|---|
| A1 | `feat(transfer): add ArchiveFormat + TransferOperation::Compress skeleton` | Enum + variante, no-op dispatcher, error claro "not yet implemented" para que el call site compile. |
| A2 | `feat(transfer): compress_pipeline for ZIP` | Pipeline + writer, integración con worker, tests ZIP. |
| A3 | `feat(transfer): compress_pipeline for TAR.GZ` | Pipeline + tests. |
| A4 | `feat(transfer): compress_pipeline for 7Z` | Pipeline + tests. |
| A5 | `refactor(transfer): migrate Compress popup to engine` | Popup encola `TransferJob::Compress`, borra `fs/ops_worker/compress.rs`. |
| A6 | `feat(transfer): add TransferOperation::Extract skeleton` | Variante + no-op dispatcher. |
| A7 | `feat(transfer): extract_pipeline for ZIP` | Pipeline + tests + sanitización de paths. |
| A8 | `feat(transfer): extract_pipeline for TAR.GZ` | Pipeline + tests. |
| A9 | `feat(transfer): extract_pipeline for 7Z` | Pipeline + tests. |
| A10 | `refactor(transfer): migrate Extract popup to engine + delete legacy compress/extract` | Popup encola `TransferJob::Extract`, borra `fs/ops_worker/{compress,extract}.rs`, borra `state.progress_rx` y el bloque 1 de `app/app/background.rs`. |

Total: 10 commits, ~ +600 líneas, ~ -250 líneas (código viejo
borrado). Tests: +12.

### 1.10 Riesgos

- **Memoria en compress:** un 7z con `Vec<u8>` intermedio
  puede ser muy grande. Mitigación: streaming directo al
  writer cuando `sevenz-rust` lo soporte; mientras tanto,
  `tokio::task::spawn_blocking` con un `BufWriter` interno.
- **Validación de paths en extract:** ya hay
  `validate_archive_entry_name` en `fs/archive.rs`. La
  reusamos; si está rota, la encontramos en esta fase y la
  arreglamos.
- **SFTP write lento:** SFTP es lento para muchas
  operaciones pequeñas. Para compress, escribimos el
  archive completo de una (no entry por entry) — eso es
  inherente al formato. Para extract sobre SSH, sí vamos a
  tener N writes pequeños. Mitigación: nada por ahora,
  SFTP v3 no soporta batch operations.

---

## Sub-proyecto 2 — Elevación (retry-as-admin) (3-4 días)

### 2.1 Estado actual

El engine retry N veces con backoff y reporta error. El
usuario tiene que abrir Pairee como admin manualmente y
reintentar. El path viejo `AdminOpKind::Rename` (que hacía
retry-as-admin para rename específico) ya fue borrado en
Fase 7.

Las primitivas que **ya existen** y vamos a reusar:

- `fs/privileges.rs::is_elevated()` — chequea si somos admin.
- `fs/privileges.rs::acquire_admin_privileges()` — re-exec el
  binario como admin (ya funciona cross-platform).
- `fs/privileges.rs::run_in_elevated_helper(ops)` — corre una
  lista de `FsOperation` en el helper elevado.

El reto: **el engine no sabe qué es "elevación"**. La idea
es que el engine sigue siendo agnóstico; el manejo de
permisos denegados vive en una capa de "policy" que detecta
el error y propone la elevación al user.

### 2.2 Diseño: separación engine / policy

Tres capas claras:

```
+-----------------------------------------------+
|  UI (TransferPanel + popup)                   |
|  - "Permission denied for X, retry as admin?" |
+-----------------------------------------------+
                  ↑ pregunta / respuesta
+-----------------------------------------------+
|  TransferPolicy (nueva, en engine)            |
|  - detecta AccessDenied en FileTransferResult|
|  - emite TransferEvent::PermissionDenied      |
|  - decide si batch-ea la pregunta al user     |
+-----------------------------------------------+
                  ↑ FileTransferResult con error
+-----------------------------------------------+
|  TransferWorker (engine, sin cambios)         |
|  - procesa files, reporta errores, sigue      |
+-----------------------------------------------+
```

`TransferPolicy` no es un objeto que vive en el worker. Es
un **trait** que se inyecta al crear el engine:

```rust
// fs/transfer/policy.rs (NUEVO)
pub trait TransferPolicy: Send + Sync + 'static {
    /// Called by the engine when a file operation fails
    /// with AccessDenied. The policy decides whether to
    /// emit PermissionDenied, batch it, etc.
    fn on_file_error(&self, error: &FileError, file: &Path);

    /// Called when the job completes. Returns the list of
    /// files that should be retried as admin.
    fn finalize(&self) -> Vec<RetryRequest>;
}

pub enum FileError {
    AccessDenied,
    NotFound,
    IoError(String),
}

pub struct RetryRequest {
    pub operation: FsOperation,  // lo que el helper elevado corre
    pub original_path: PathBuf,
    pub error: String,
}
```

La impl por default (`LoggingPolicy`) solo loguea warnings.
La impl de producción (`PromptPolicy`) usa un
`mpsc::Sender<PermissionEvent>` para comunicarse con la UI.

### 2.3 Flujo de usuario

1. User dispara Copy/Move/Delete/etc.
2. Engine procesa archivos, uno por uno.
3. Archivo X falla con `AccessDenied`. La policy emite
   `TransferEvent::PermissionDenied { job_id, file, error }`
   y registra el file en su lista interna.
4. Engine **continúa** con el siguiente archivo (no pausa).
5. Al final del job, la policy mira cuántos archivos
   fallaron por permisos. Si es > 0, emite
   `TransferEvent::PermissionPrompt { job_id, count, files }`.
6. La UI recibe el prompt y muestra un popup: "5 archivos
   fallaron por permisos. ¿Reintentar como administrador?"
   con botones [Sí] [No] [Cancelar todo].
7. User elige Sí: la policy construye la lista de
   `RetryRequest`s y llama a `run_in_elevated_helper(ops)`.
8. Después del helper, la UI refresca los paneles y marca
   los files como completed en el job's results.
9. Si el user eligió No: los files quedan en failed, se
   reporta al final normalmente.
10. Si el user eligió Cancelar: idem + abortar el job si
    todavía está corriendo.

### 2.4 Type-safety: `PermissionPrompt` es un evento nuevo

```rust
// fs/transfer/events.rs
pub enum TransferEvent {
    // ... los que ya están ...
    PermissionDenied { job_id: Uuid, file: PathBuf, error: String },
    PermissionPrompt { job_id: Uuid, count: usize, files: Vec<PathBuf> },
}
```

Esto es importante: el engine **no pausa** al primer error.
La pregunta se hace una sola vez al final. Si hay 100
archivos con permisos denegados, el user responde una vez
por el lote.

### 2.5 Compatibilidad con SSH

La elevación **no aplica a operaciones sobre SSH**. El
helper elevado corre en la máquina local, no puede
re-ejecutar operaciones en un server remoto. Si la policy
ve que el endpoint es `TransferEndpoint::Ssh`, los
`AccessDenied` se reportan como error normal (el user
debe arreglar los permisos en el server, no en su
máquina).

### 2.6 Compatibilidad con MkDir, Rename, CreateLink, Wipe

`FsOperation` ya tiene variantes para Copy/Move/Delete/
MkDir. Le agregamos `Rename` y `CreateLink`:

```rust
// fs/privileges.rs
pub enum FsOperation {
    Copy { src: PathBuf, dst: PathBuf },
    Move { src: PathBuf, dst: PathBuf },
    Delete { path: PathBuf },
    MkDir { path: PathBuf },
    Rename { src: PathBuf, dst: PathBuf },
    CreateLink { src: PathBuf, dst: PathBuf, kind: LinkKind },
    // Wipe: ver §2.7
}
```

`Wipe` no es una `FsOperation` actualmente. Lo agregamos o
lo manejamos como `Delete` en el helper (con flag
`secure_erase`).

### 2.7 Tests

| Test | Cubre |
|---|---|
| `policy_logging_does_not_pause_engine` | LoggingPolicy deja que el engine siga. |
| `policy_prompt_batches_errors` | 5 AccessDenied → 1 PermissionPrompt, no 5. |
| `policy_ignores_ssh_access_denied` | Ssh + AccessDenied → FileError::IoError, no Prompt. |
| `policy_creates_retry_requests_for_local_only` | Solo archivos Local van a la lista de retry. |
| `engine_continues_after_access_denied` | 10 archivos, 3 con permisos, los 7 restantes se copian. |
| `transfer_event_permission_prompt_includes_count` | El evento lleva el count correcto. |

### 2.8 Fases (código)

| # | Commit | Contenido |
|---|---|---|
| B1 | `feat(transfer): add TransferPolicy trait + LoggingPolicy default` | Trait + impl default + tests. |
| B2 | `feat(transfer): add FileError + RetryRequest types` | Types + tests. |
| B3 | `feat(transfer): integrate policy into TransferWorker` | Worker invoca policy.on_file_error, emite nuevos eventos. |
| B4 | `feat(transfer): add PermissionPrompt popup` | UI nueva `PopupType::PermissionPrompt` con 3 botones. |
| B5 | `feat(transfer): wire PermissionPrompt to run_in_elevated_helper` | Click "Sí" construye `FsOperation`s y llama al helper. |
| B6 | `feat(transfer): expand FsOperation with Rename + CreateLink + Wipe` | Helper soporta más operaciones. |
| B7 | `feat(transfer): delete legacy progress_rx machinery` | Fase 10 ya lo hizo; verificamos que no quede nada. |

Total: 7 commits, ~ +500 líneas, ~ -50 líneas. Tests: +6.

### 2.9 Riesgos y mitigaciones

- **Race condition:** el engine termina el job y la UI
  tarda en mostrar el prompt. Si el user hace otra
  operación entre medio, ¿qué pasa? Mitigación: el
  `PermissionPrompt` lleva `job_id`, la UI solo lo aplica
  al job que lo pidió.
- **Helper elevado falla:** ¿qué hace el engine? El
  engine ya terminó; la policy reporta el fallo como
  "elevated retry failed" en el log del job. No se vuelve
  a intentar.
- **Doble elevación:** si el user ya está elevado
  (`is_elevated()`), `acquire_admin_privileges` es no-op.
  La policy lo detecta y no muestra el prompt
  innecesariamente.
- **UX confuso:** "5 archivos fallaron" — ¿cuáles?
  Mitigación: el popup lista los archivos (scrollable si
  son muchos).
- **Cross-platform:** `acquire_admin_privileges` ya está
  implementado para Windows (UAC) y Unix (`sudo`). El
  helper elevado ya funciona. No reescribimos.

---

## Sub-proyecto 3 — Direct I/O sobre SSH: ya está manejado

El motor nuevo (`copy_file_pipelined`) ya detecta
`local_direct_io` correctamente:

```rust
// fs/transfer/pipeline.rs línea ~28
let local_direct_io = options.direct_io
    && src_endpoint.is_local()
    && dst_endpoint.is_local();
```

Si cualquiera de los dos endpoints es `Ssh`, el flag se
apaga y se usa I/O estándar. No hay nada que cambiar.

**UI:** la opción `direct_io` está en `TransferOptions` y
la UI la muestra como un toggle. No hay UI "vieja" de
Direct I/O separada — ya está toda en el motor.

**Acción:** nada. Confirmado.

---

## Sub-proyecto 4 — xattr / ACL / ADS sobre SSH: ya está manejado

Decisión original (Fase 1 confirmada): **SFTP nativo
solamente, no `session.exec`**. El código ya lo respeta:

- `preserve_metadata` en `fs/transfer/metadata.rs` solo
  llama a `endpoint.set_xattr/list_xattrs/get_xattr` cuando
  `endpoint.supports_xattr()` devuelve `true` (que es solo
  para `Local`).
- ACL/ADS: el código está en bloques `#[cfg(windows)]` y
  chequea `is_local()` antes de invocar las APIs de Windows.

Si el endpoint es `Ssh`, se loguea un warning y se sigue.
El archivo se copia, simplemente sin esa metadata
extendida.

**Acción:** nada. Confirmado. Si en el futuro queremos
shell-out, lo agregamos con un feature flag
(`cfg(feature = "ssh-metadata-shellout")`) para no
romper la decisión de diseño.

---

## Sobre SFTP v6 hardlinks (pregunta 4)

🔴 **DEFERIDO — PENDIENTE PARA FUTURO.**

**Razones:**

1. La mayoría de los servers (OpenSSH clásico, la mayoría de
   NAS, casi todo lo que vas a encontrar) están en SFTP v3.
   Soportar v6 nos deja igual que ahora en el 95% de los
   casos reales.
2. Implementarlo lleva 1-2 días + conseguir un server v6
   para testear (no es trivial — la mayoría de servers
   públicos de prueba siguen en v3).
3. Si en 1-2 años v6 se masifica (cosa que no se está
   viendo todavía), lo agregamos con un feature flag sin
   romper nada.

**Cuándo se levanta este ticket:**

- Cuando el usuario reporte un caso de uso real donde
  necesita hardlinks en un server remoto.
- Cuando OpenSSH 10+ o algún server mainstream marque v6
  como default (monitorear releases).
- Si en algún momento se decide soportar SMB/NFS sobre
  SSH como endpoint nuevo, donde v6 es más relevante.

**Estimado cuando se implemente:**

- ~1-2 días de trabajo, ~150 líneas.
- Enviar un `LINK` raw a través de `Sftp::send_request` (la
  API de bajo nivel de ssh2).
- Feature flag propuesto: `ssh-v6` en `Cargo.toml` para no
  romper la decisión actual.
- Tests contra un server v6 real (difícil de conseguir —
  verificar antes de empezar).

---

## Orden de ejecución propuesto

Si todo esto te parece bien, arrancamos en este orden:

1. **Sub-proyecto 2 (Elevación)** — porque es el más
   "infraestructura" y sienta las bases de policy
   reutilizables.
2. **Sub-proyecto 1 (Compress/Extract)** — el más grande,
   necesita las crates que ya tenemos.
3. **Confirmación de 3 y 4** — solo changelog, no código.

Total estimado: 9-12 días de trabajo concentrado, 17
commits, 18 tests nuevos, 305 tests totales al final.

---

## Riesgos globales

- **Scope creep:** la tentación será agregar features
  nuevas (multi-volume 7z, password, encryption, etc.)
  mientras estamos en el archivo. **Regla:** si no estaba
  en el código viejo, no lo agregamos.
- **Compatibilidad de versiones de las crates de
  formato:** `sevenz-rust` está en 0.6.1 y es
  relativamente inmadura. Si rompe una API, parcheamos
  localmente o subimos versión.
- **Performance del helper elevado en Unix:** `sudo -A`
  pregunta por password si no hay cache. Documentamos
  cómo configurar `NOPASSWD` en sudoers para que sea
  transparente.
- **Tests SSH:** seguimos sin server SSH en CI. La
  integración SSH se prueba manualmente.

---

## Criterio de "done"

- [ ] Las 7 fases de elevación commiteadas, 6 tests nuevos
      pasando.
- [ ] Las 10 fases de compress/extract commiteadas, 12
      tests nuevos pasando.
- [ ] `cargo build`, `cargo clippy --all-targets -- -D
      warnings`, `cargo test --all`, `cargo fmt --check`
      todos limpios.
- [ ] Smoke test manual (procedimiento completo en
      `docs/technical/smoke-test.md`):
  - [ ] Compress local → local
  - [ ] Compress local → SSH
  - [ ] Compress SSH → local
  - [ ] Extract local → local
  - [ ] Extract SSH → local
  - [ ] Copy archivo de Sistema en Windows como admin
        (elevación) — incluye el chequeo de que el
        popup muestra el `sample_error` (L3)
  - [ ] Copy archivo de Sistema en Linux con `sudo` —
        incluye el chequeo de que el temp file es 0600
        (C1)
  - [ ] Copy normal (no elevación) sigue funcionando
  - [ ] Compress ZIP level=0 → archivo al menos tan
        grande como la suma de los sources (L2 sanity)
  - [ ] Compress con archivo 0o600 → entry preserva
        0o600 (L2)
- [ ] CHANGELOG actualizado.
