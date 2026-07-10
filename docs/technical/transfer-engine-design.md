# Pairee Transfer Engine — Documento de Diseño Técnico

> **Rama:** `feature/transfer-engine`
> **Tipo:** Feature + Reforge del subsistema de copia/movimiento
> **Versión objetivo:** 0.7.0
> **Estado:** En planificación

---

## Tabla de Contenidos

1. [Resumen Ejecutivo](#1-resumen-ejecutivo)
2. [Funcionalidades a Implementar](#2-funcionalidades-a-implementar)
3. [Patrones de Diseño Utilizados](#3-patrones-de-diseño-utilizados)
4. [Arquitectura del Sistema](#4-arquitectura-del-sistema)
5. [Librerías a Utilizar](#5-librerías-a-utilizar)
6. [Cambios en la UI y Flujo de Trabajo](#6-cambios-en-la-ui-y-flujo-de-trabajo)
7. [Plan de Implementación Detallado](#7-plan-de-implementación-detallado)

---

## 1. Resumen Ejecutivo

Este documento describe el reforje completo del subsistema de copia/movimiento de Pairee para convertirlo en un **Transfer Engine** de clase profesional, inspirado en TeraCopy Pro. El motor actual (`fs::ops_worker`) soporta una sola operación de copia/movimiento a la vez, con un popup modal simple (`CopyProgress`) que bloquea la interfaz y no permite interacción con los paneles durante la transferencia.

### Limitaciones actuales del sistema

| Aspecto | Estado actual | Estado objetivo |
|---------|--------------|-----------------|
| Operaciones simultáneas | Una sola (`progress_rx: Option<Receiver>`) | Cola ilimitada + concurrencia configurable |
| UI durante copia | Popup modal bloqueante | Barra minimizable + popup expandible |
| Verificación de integridad | No existe | CRC32, MD5, SHA-1, SHA-256, BLAKE3 |
| Pausa/reanudación | No soportado | Completo con `CancellationToken` |
| Recuperación de errores | Aborta toda la operación | Reintento por archivo + skip |
| Movimiento seguro | Copia + delete sin verificación | Hash pre-move → copy → verify → delete |
| Informes | No existe | HTML + CSV exportable |
| Filtros | Limitados | Glob include/exclude completo |
| Metadatos | No preservados en copia manual | Timestamps + permisos + ADS |

### Impacto en archivos existentes

El reforje afecta los siguientes módulos:

- `src/fs/ops_worker/` — Reescritura completa
- `src/app/state/types.rs` — Nuevos tipos de estado
- `src/app/state/mod.rs` — Nuevo campo `transfer_queue`
- `src/app/app/background.rs` — Nuevo procesador de cola
- `src/ui/layout.rs` — Barra de transferencia
- `src/ui/popup/` — Nuevos popups de transferencia
- `src/keybindings/actions.rs` — Nuevas acciones
- `src/config/settings.rs` — Nuevos ajustes de transferencia
- `src/config/localization/en.rs` — Nuevas claves de texto

---

## 2. Funcionalidades a Implementar

### 2.1. Motor de Copia Directa (Bypass Cache)

Transferencia de archivos sin almacenamiento en caché del sistema operativo para reducir los tiempos de búsqueda y evitar contaminar el caché del sistema con datos de archivos grandes.

**Detalles técnicos:**
- En Windows: Usar flag `FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH` vía `CreateFileW`
- En Linux: Usar flag `O_DIRECT` vía `open(2)` + alinear buffers a sectores de 4096 bytes
- Buffer configurable: 64KB, 256KB, 1MB, 4MB (seleccionable por el usuario)
- Fallback automático a copia con buffer estándar si el SO no soporta direct I/O

### 2.2. Pausa y Reanudación

Control completo del flujo de transferencia.

**Detalles técnicos:**
- Cada tarea de transferencia recibe un `tokio_util::sync::CancellationToken` para cancelación
- Cada tarea monitorea un `Arc<AtomicBool>` para pausa (`is_paused`)
- Cuando `is_paused == true`, el loop de lectura/escritura entra en `tokio::time::sleep(100ms)` polling
- El estado de bytes copiados persiste para reanudación intra-archivo (seek al byte exacto)
- UI: Botón `[Pause]` ↔ `[Resume]` en el panel de transferencia

### 2.3. Recuperación de Errores

Tolerancia a errores a nivel de archivo individual.

**Detalles técnicos:**
- Configurar `max_retries: u32` (default 3) en `TransferSettings`
- En caso de error de lectura de un archivo:
  1. Reintentar `max_retries` veces con backoff exponencial (100ms, 500ms, 2s)
  2. Si persiste el error, registrar en `TransferResult::failed_files`
  3. Continuar con el siguiente archivo (nunca abortar la operación completa)
- Opción "Halt on first error" para comportamiento legacy
- Errores de escritura (disco lleno, permisos): prompt inmediato al usuario con opciones Skip/Retry/Abort

### 2.4. Lista de Archivos Fallidos

Visualización y re-procesamiento de archivos que fallaron.

**Detalles técnicos:**
- Struct `FailedFile { src: PathBuf, dst: PathBuf, error: String, retries: u32 }`
- Lista accesible desde el popup de transferencia expandido (pestaña "Errors")
- Opción "Retry Failed" que re-encola solo los archivos fallidos como un nuevo `TransferJob`
- Exportable a CSV/HTML junto con el informe general

### 2.5. Verificación de Integridad (Hash)

Comparación de archivos de origen y destino mediante hash criptográfico.

**Detalles técnicos:**
- Algoritmos soportados: CRC32, MD5, SHA-1, SHA-256, BLAKE3
- Configuración por defecto: BLAKE3 (el más rápido para archivos grandes)
- Modos de verificación:
  - **Post-copy:** Calcular hash del destino después de copiar, comparar con hash del origen
  - **Asíncrona (pipeline):** Calcular hash del origen *mientras* se lee el bloque, y del destino *mientras* se escribe el siguiente (ver sección 2.17)
- Buffer compartido para evitar re-lectura del archivo origen
- Resultado almacenado en `FileTransferResult { src_hash: String, dst_hash: String, match: bool }`
- Discrepancias mostradas con iconos visuales (✓ / ✗ / ⚠) en la lista de archivos

### 2.6. Movimiento Seguro con Verificación

Para operaciones de "mover" cuando el usuario activa verificación.

**Flujo:**
1. Calcular hash del archivo origen → `src_hash`
2. Copiar archivo origen → destino
3. Calcular hash del archivo destino → `dst_hash`
4. Si `src_hash == dst_hash`: eliminar archivo origen
5. Si `src_hash != dst_hash`: NO eliminar origen, registrar error "Hash mismatch", marcar archivo como fallido

**Detalles técnicos:**
- Este modo reemplaza el `fs::rename()` fast-path actual para movimiento en misma partición cuando la verificación está activa
- Cross-device moves ya usan copy+delete; solo se agrega la fase de verificación
- Indicador visual diferente en la UI: icono de candado (🔒) junto al archivo durante la fase de verificación

### 2.7. Gestión de Cola de Transferencias

Sistema de encolamiento para múltiples paquetes de transferencia.

**Detalles técnicos:**
- Struct `TransferQueue` con `VecDeque<TransferJob>` protegido por `Arc<Mutex<_>>`
- Cada `TransferJob` contiene:
  - `id: Uuid` — Identificador único
  - `operation: TransferOperation` (Copy/Move)
  - `sources: Vec<PathBuf>`
  - `destination: PathBuf`
  - `options: TransferOptions` (verificación, filtros, etc.)
  - `status: TransferJobStatus` (Queued/Running/Paused/Completed/Failed)
  - `progress: TransferProgress`
  - `created_at: Instant`
- Modo de ejecución: Secuencial (una a la vez) por defecto, con opción de paralelismo configurable
- Desde la UI, el usuario puede:
  - Agregar trabajos a la cola desde F5/F6
  - Reordenar trabajos en la cola (move up/down)
  - Eliminar trabajos pendientes de la cola
  - Ver estado de cada trabajo

### 2.8. Preservación de Metadatos

Mantener atributos originales de archivos.

**Detalles técnicos:**
- Metadatos a preservar:
  - `created_at` (timestamp de creación)
  - `modified_at` (timestamp de modificación)
  - `accessed_at` (timestamp de acceso)
  - Permisos (Unix `mode_t` / Windows ACLs)
  - Atributos de archivo (Windows: hidden, system, readonly, etc.)
  - Alternate Data Streams (Windows NTFS) — opcional
- Implementación:
  - Windows: `SetFileTime()` + `SetFileAttributesW()` via `windows-sys`
  - Linux: `filetime` crate + `std::fs::set_permissions()`
- Checkboxes individuales en la UI de opciones de transferencia (como TeraCopy):
  - ☑ Data
  - ☑ Timestamps
  - ☑ Attributes
  - ☑ Streams (Windows only)
  - ☐ Security / ACL
  - ☐ Owner

### 2.9. Soporte de Nombres Largos (Long Paths)

Manejar rutas que superen el límite estándar de 260 caracteres de Windows.

**Detalles técnicos:**
- Windows: Prefijar rutas con `\\?\` automáticamente cuando `path.len() > 248`
- Usar `std::path::PathBuf` con la extensión `\\?\` prefix
- En el manifest de la app: `longPathAware = true`
- Afecta: `CreateFileW`, `CreateDirectoryW`, `MoveFileExW`, `DeleteFileW`
- No afecta Linux (límite de 4096 chars normalmente suficiente)

### 2.10. Acciones Post-Procesamiento

Ejecutar acciones automáticas al terminar todos los trabajos de la cola.

**Detalles técnicos:**
- Enum `PostAction`:
  - `None` — No hacer nada
  - `Shutdown` — Apagar el PC (`shutdown /s /t 0` / `shutdown -h now`)
  - `Sleep` — Suspender (`rundll32.exe powrprof.dll,SetSuspendState` / `systemctl suspend`)
  - `Hibernate` — Hibernar
  - `EjectDrive(String)` — Expulsar unidad específica
  - `RunScript(PathBuf)` — Ejecutar script personalizado
  - `CloseApp` — Cerrar Pairee
- Configuración en el popup de opciones de la cola, NO en settings globales
- Confirmación de seguridad antes de ejecutar acciones destructivas (shutdown/hibernate)

### 2.11. Informes Detallados

Exportación de resultados de transferencia.

**Detalles técnicos:**
- Formatos: HTML y CSV
- Contenido del informe:
  - Fecha y hora de inicio/fin
  - Duración total
  - Total de archivos procesados, exitosos, fallidos, omitidos
  - Total de bytes transferidos
  - Velocidad media
  - Lista detallada de cada archivo: ruta origen, ruta destino, tamaño, estado, hash (si aplica), error (si aplica)
- Ubicación por defecto: `%APPDATA%/pairee/cache/reports/` (Windows) o `~/.cache/pairee/reports/` (Linux)
- Template HTML embebido como `include_str!` en el binario
- Generación automática opcional o bajo demanda (botón "Export Report" en el popup)

### 2.12. Edición de Tareas en Cola

Permitir modificar la cola durante la ejecución.

**Detalles técnicos:**
- Desde el popup expandido de transferencia (pestaña "Queue"):
  - Eliminar archivos individuales de un trabajo pendiente
  - Eliminar trabajos completos de la cola
  - Cambiar prioridad (mover arriba/abajo)
  - Editar destino de un trabajo pendiente
- Archivos que ya están siendo transferidos no pueden eliminarse
- Protección con `Mutex`: las ediciones bloquean brevemente la cola pero no el worker

### 2.13. Omisión de Enlaces Simbólicos

Configuración para saltarse automáticamente symlinks y junctions.

**Detalles técnicos:**
- Opciones en `TransferOptions`:
  - `skip_symlinks: bool` — Omitir enlaces simbólicos
  - `skip_junctions: bool` — Omitir junctions de directorio (Windows)
  - `follow_symlinks: bool` — Seguir symlinks y copiar el target (en vez del link)
- Detección: `std::fs::symlink_metadata()` + verificar `file_type().is_symlink()`
- En Windows: Detectar junctions via `FILE_ATTRIBUTE_REPARSE_POINT` + comprobar reparse tag
- Default: `skip_symlinks = false`, `follow_symlinks = false` (copiar el link tal cual, como el sistema actual)

### 2.14. Filtros de Archivos

Incluir o excluir tipos de archivos por patrón.

**Detalles técnicos:**
- Patrones glob: `*.jpg;*.png;*.gif` (include) o `!*.tmp;!*.bak` (exclude)
- Reutilizar la función existente `glob_matches()` de `src/app/state/glob.rs`
- Interfaz en el popup de confirmación de copia/movimiento (ya existe campo `filter_mask` en `CopyPrompt`)
- Ampliar para soportar:
  - Filtros por tamaño: `>10MB`, `<1KB`
  - Filtros por fecha: `newer:2024-01-01`, `older:30d`
- Los filtros se aplican durante la fase de escaneo (no después de copiar)

### 2.15. Renombrado Inteligente de Conflictos

Resolución automática de nombres duplicados.

**Detalles técnicos:**
- Enum `ConflictResolution`:
  - `Ask` — Preguntar al usuario por cada conflicto
  - `Overwrite` — Sobrescribir siempre
  - `OverwriteOlder` — Sobrescribir solo si el origen es más nuevo
  - `Skip` — Omitir archivo
  - `Rename` — Renombrar automáticamente
  - `KeepBoth` — Mantener ambos (renombrar el nuevo)
- Patrón de renombrado: `filename (1).ext`, `filename (2).ext`, etc.
- Opción "Apply to all" para no preguntar archivo por archivo
- Detección: Verificar existencia en destino *antes* de iniciar la copia del archivo

### 2.16. Soporte de Red (LAN)

Optimización para transferencias de red.

**Detalles técnicos:**
- Detección automática de si el destino es una unidad de red: `GetDriveTypeW()` en Windows, `/proc/mounts` en Linux
- Ajustes automáticos para red:
  - Buffer más grande (1MB en vez de 64KB)
  - Timeout de conexión configurable
  - Reintentos automáticos en desconexiones temporales (hasta 60s de espera)
- Throttling opcional: limitar ancho de banda (bytes/s) para no saturar la red
- Verificación de espacio libre en destino antes de iniciar (especialmente importante en red)

### 2.17. Verificación Asíncrona (Pipeline)

Calcular hashes en paralelo con la I/O para maximizar rendimiento.

**Detalles técnicos:**
- Arquitectura de pipeline de 3 etapas:
  1. **Lector:** Lee bloques del archivo origen y los envía al canal
  2. **Hasher origen:** Consume bloques del lector, calcula hash incremental
  3. **Escritor + Hasher destino:** Escribe bloques al destino, calcula hash del destino simultáneamente
- Implementación con `tokio::sync::mpsc` para comunicar bloques entre etapas
- Tamaño de bloque: Configurable (default 1MB para maximizar throughput)
- Reducción de re-lectura: El archivo origen se lee UNA SOLA VEZ

```text
┌──────────┐    ┌──────────────────┐    ┌──────────────────┐
│  Lector  │───▶│ Hasher (origen)  │───▶│ Escritor + Hash  │
│ (read)   │    │ BLAKE3 streaming │    │ (write + hash)   │
└──────────┘    └──────────────────┘    └──────────────────┘
     ▲                                         │
     │         canal mpsc (bloques)            │
     └─ archivo origen           archivo destino ─┘
```

### 2.18. Historial de Carpetas Origen/Destino

Registro de rutas utilizadas recientemente para accesos rápidos.

**Detalles técnicos:**
- Almacenar las últimas 20 rutas de origen y 20 rutas de destino en `transfer_history.toml`
- Ubicación: junto con la configuración de la app (`%APPDATA%/pairee/config/`)
- Accesible desde el popup de copia/movimiento como dropdown/autocompletado
- Integración con el sistema de historial existente (`config::history`)

### 2.19. Soporte Unicode Completo

Manejo de caracteres especiales en nombres de archivo.

**Detalles técnicos:**
- Rust ya maneja Unicode nativamente via `OsString` / `PathBuf`
- Asegurar que los informes HTML/CSV se exporten como UTF-8 con BOM (para Excel)
- Verificar que los hashes se calculen correctamente independientemente del encoding del nombre
- Normalización NFC/NFD para comparación de nombres en destino (evitar falsos negativos)

---

## 3. Patrones de Diseño Utilizados

### 3.1. Actor Model (para workers de transferencia)

Cada trabajo de transferencia se ejecuta como un **actor aislado** dentro de un `tokio::task`. El actor se comunica con el hilo principal exclusivamente mediante canales (`mpsc`), sin estado compartido mutable.

```text
┌─────────────────┐      mpsc::channel       ┌──────────────────┐
│  Transfer Worker │ ──────────────────────▶  │  AppState (main) │
│  (tokio::task)   │  TransferEvent          │  (event loop)    │
│                  │ ◀──────────────────────  │                  │
│                  │  TransferCommand         │                  │
└─────────────────┘  (pause/cancel/skip)     └──────────────────┘
```

**Justificación:** Elimina race conditions y simplifica testing (el worker se puede testear independientemente).

### 3.2. Command Pattern (para cola de transferencias)

Cada operación de transferencia se encapsula como un **comando inmutable** (`TransferJob`) que contiene toda la información necesaria para ejecutarse. La cola es simplemente una `VecDeque<TransferJob>`.

**Justificación:** Permite serializar, reordenar, cancelar y reintentar trabajos de forma trivial.

### 3.3. Strategy Pattern (para algoritmos de hash)

Los algoritmos de verificación implementan un trait `HashStrategy`:

```rust
pub trait HashStrategy: Send + Sync {
    fn name(&self) -> &str;
    fn update(&mut self, data: &[u8]);
    fn finalize(self) -> String;
    fn clone_box(&self) -> Box<dyn HashStrategy>;
}
```

**Justificación:** Añadir un nuevo algoritmo es simplemente implementar el trait, sin modificar código existente (Open/Closed Principle).

### 3.4. Observer Pattern (para eventos de progreso)

El worker emite `TransferEvent` a través de un canal. Múltiples observadores pueden suscribirse:
- La UI (para actualizar la barra de progreso)
- El logger (para escribir en el informe)
- El sistema de notificaciones (para post-acciones)

### 3.5. State Machine (para estados de transferencia)

Cada `TransferJob` sigue una máquina de estados finita:

```text
Queued ──▶ Scanning ──▶ Transferring ──▶ Verifying ──▶ Completed
  │           │             │               │             │
  │           ▼             ▼               ▼             ▼
  └──▶ Cancelled    Paused ◀──▶ Transferring   Failed   
                       │
                       └──▶ Cancelled
```

### 3.6. Builder Pattern (para TransferOptions)

Configuración fluida de las opciones de transferencia:

```rust
TransferOptions::builder()
    .verify_hash(HashAlgorithm::Blake3)
    .buffer_size(BufferSize::_1MB)
    .conflict_resolution(ConflictResolution::OverwriteOlder)
    .preserve_timestamps(true)
    .skip_symlinks(true)
    .filter("*.jpg;*.png")
    .max_retries(3)
    .build()
```

### 3.7. Pipeline Pattern (para verificación asíncrona)

El pipeline de lectura → hash → escritura se implementa con canales `mpsc` que actúan como buffers intermedios, permitiendo que las tres etapas operen concurrentemente sin blocking.

---

## 4. Arquitectura del Sistema

### 4.1. Diagrama de Módulos

```text
src/
├── fs/
│   └── transfer/                    # [NUEVO] Motor de transferencia
│       ├── mod.rs                   # API pública del motor
│       ├── engine.rs                # TransferEngine: orquestador principal
│       ├── queue.rs                 # TransferQueue: cola de trabajos
│       ├── job.rs                   # TransferJob: definición del trabajo
│       ├── worker.rs               # Worker: ejecución del trabajo individual
│       ├── pipeline.rs             # Pipeline de lectura/hash/escritura
│       ├── hash/                   # Algoritmos de hash
│       │   ├── mod.rs              # Trait HashStrategy + factory
│       │   ├── crc32.rs            # CRC32
│       │   ├── md5.rs              # MD5
│       │   ├── sha1.rs             # SHA-1
│       │   ├── sha256.rs           # SHA-256
│       │   └── blake3.rs           # BLAKE3
│       ├── options.rs              # TransferOptions + builder
│       ├── events.rs               # TransferEvent enum (progreso, errores, etc.)
│       ├── conflict.rs             # Resolución de conflictos de nombres
│       ├── filter.rs               # Filtros de archivos (glob, size, date)
│       ├── metadata.rs             # Preservación de metadatos
│       ├── report.rs               # Generación de informes HTML/CSV
│       ├── direct_io.rs            # I/O directa (bypass cache)
│       ├── network.rs              # Detección y optimización de red
│       └── post_action.rs          # Acciones post-procesamiento
│
├── app/
│   └── state/
│       └── transfer_state.rs       # [NUEVO] Estado de UI del transfer engine
│
├── ui/
│   └── transfer/                   # [NUEVO] Componentes de UI de transferencia
│       ├── mod.rs                  # Exportaciones
│       ├── bar.rs                  # Barra minimizada (bottom bar)
│       ├── panel.rs                # Panel expandido (popup completo)
│       ├── queue_view.rs           # Vista de cola de trabajos
│       ├── file_list.rs            # Lista de archivos (copiados/fallidos)
│       ├── options_view.rs         # Vista de opciones de transferencia
│       └── report_view.rs          # Vista previa de informe
│
├── config/
│   └── settings.rs                 # [MODIFICAR] Nuevos campos de TransferSettings
│
└── keybindings/
    └── actions.rs                  # [MODIFICAR] Nuevas acciones de transferencia
```

### 4.2. Diagrama de Flujo de Datos

```text
┌─────────────┐     F5/F6     ┌──────────────────┐
│  Panel UI   │ ─────────────▶│ TransferOptions   │
│  (copiar/   │               │ dialog (popup)    │
│   mover)    │               └───────┬───────────┘
└─────────────┘                       │ confirmar
                                      ▼
                            ┌──────────────────┐
                            │  TransferEngine  │
                            │  (orquestador)   │
                            └───────┬──────────┘
                                    │ encolar
                                    ▼
┌──────────────────┐  dequeue  ┌──────────────────┐
│  TransferQueue   │ ─────────▶│  TransferWorker  │
│  (VecDeque)      │           │  (tokio::task)   │
└──────────────────┘           └───────┬──────────┘
                                       │ progreso
                                       ▼
                    ┌──────────────────────────────────┐
                    │        TransferEvent channel      │
                    │  (mpsc: worker → main loop)      │
                    └───────┬───────┬───────┬──────────┘
                            │       │       │
                            ▼       ▼       ▼
                    ┌───────┐ ┌─────┐ ┌──────────┐
                    │  UI   │ │ Log │ │ PostAct  │
                    │ (bar) │ │     │ │ (report) │
                    └───────┘ └─────┘ └──────────┘
```

### 4.3. Modelo de Estado

```rust
// En src/fs/transfer/job.rs
pub struct TransferJob {
    pub id: uuid::Uuid,
    pub operation: TransferOperation,
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    pub options: TransferOptions,
    pub status: TransferJobStatus,
    pub progress: TransferProgress,
    pub results: TransferResults,
    pub created_at: std::time::Instant,
    pub started_at: Option<std::time::Instant>,
    pub completed_at: Option<std::time::Instant>,
}

pub enum TransferOperation { Copy, Move }

pub enum TransferJobStatus {
    Queued,
    Scanning,
    Transferring,
    Verifying,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

pub struct TransferProgress {
    pub current_file: String,
    pub files_scanned: usize,
    pub files_total: usize,
    pub files_completed: usize,
    pub files_failed: usize,
    pub files_skipped: usize,
    pub bytes_total: u64,
    pub bytes_transferred: u64,
    pub bytes_per_second: f64,
    pub eta_seconds: Option<u64>,
}

pub struct TransferResults {
    pub completed_files: Vec<FileTransferResult>,
    pub failed_files: Vec<FailedFile>,
    pub skipped_files: Vec<SkippedFile>,
}

pub struct FileTransferResult {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub size: u64,
    pub src_hash: Option<String>,
    pub dst_hash: Option<String>,
    pub verified: bool,
    pub duration: std::time::Duration,
}

pub struct FailedFile {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub error: String,
    pub retries: u32,
}
```

### 4.4. Modelo de Comunicación

```rust
// Worker → Main (eventos)
pub enum TransferEvent {
    JobStarted { job_id: Uuid },
    ScanProgress { job_id: Uuid, files_found: usize },
    ScanComplete { job_id: Uuid, total_files: usize, total_bytes: u64 },
    FileStarted { job_id: Uuid, file: PathBuf, index: usize },
    FileProgress { job_id: Uuid, bytes_copied: u64, bytes_total: u64 },
    FileCompleted { job_id: Uuid, result: FileTransferResult },
    FileFailed { job_id: Uuid, error: FailedFile },
    FileSkipped { job_id: Uuid, file: PathBuf, reason: String },
    VerifyStarted { job_id: Uuid },
    VerifyProgress { job_id: Uuid, files_verified: usize, total: usize },
    JobCompleted { job_id: Uuid, results: TransferResults },
    JobFailed { job_id: Uuid, error: String },
    SpeedUpdate { job_id: Uuid, bytes_per_second: f64, eta_seconds: Option<u64> },
    ConflictDetected { job_id: Uuid, file: PathBuf, conflict: ConflictInfo },
}

// Main → Worker (comandos)
pub enum TransferCommand {
    Pause,
    Resume,
    Cancel,
    SkipFile,
    RetryFile,
    ResolveConflict { resolution: ConflictResolution },
}
```

---

## 5. Librerías a Utilizar

### Nuevas dependencias

| Crate | Versión | Propósito | Justificación |
|-------|---------|-----------|---------------|
| `blake3` | `1.x` | Hash BLAKE3 | Rendimiento superior a SHA-256, paralelizable nativamente |
| `md-5` | `0.10` | Hash MD5 | Compatibilidad con herramientas legacy |
| `sha1` | `0.10` | Hash SHA-1 | Compatibilidad con herramientas legacy |
| `sha2` | `0.10` | Hash SHA-256 | Verificación estándar de la industria |
| `crc32fast` | `1.x` | Hash CRC32 | Verificación rápida y ligera |
| `uuid` | `1.x` | IDs únicos para trabajos | Identificación inequívoca de cada TransferJob |
| `filetime` | `0.2` | Preservar timestamps | Cross-platform metadata preservation |
| `bytesize` | `1.x` | Formateo de tamaños | Display "1.3 GB", "456 KB" en la UI |

### Dependencias existentes reutilizadas

| Crate | Uso actual | Uso ampliado |
|-------|-----------|--------------|
| `tokio` | Tareas async background | Workers de transferencia + pipeline |
| `serde` + `toml` | Configuración | Serialización de TransferOptions e historial |
| `ratatui` | UI TUI | Nuevos widgets de barra y panel de transferencia |
| `crossterm` | Eventos de teclado | Nuevas teclas para pausa/expandir/minimizar |
| `anyhow` | Error handling | Error context en workers |
| `log` | Logging | Logging de eventos de transferencia |
| `chrono` | Timestamps | Fechas en informes |

---

## 6. Cambios en la UI y Flujo de Trabajo

### 6.1. Flujo de Trabajo del Usuario

#### Flujo básico: Copiar archivos

```text
1. Usuario selecciona archivos en panel activo
2. Presiona F5 (Copy) o F6 (Move)
3. Se abre popup de confirmación (REFORJADO):
   ┌────────────────────────────────────────────────────┐
   │  Copy 5 files to D:\Backup\                       │
   │  ──────────────────────────────────────────────    │
   │  Destination: [D:\Backup\__________________]  [▼] │
   │                                                    │
   │  [File List] [Options] [Queue]                     │
   │                                                    │
   │  Transfer options:                                 │
   │    ☑ Verify files after transfer    Hash: [BLAKE3] │
   │    ☐ Direct I/O (bypass cache)                     │
   │    Buffer: [1 MB ▼]                                │
   │                                                    │
   │  On conflict: [Overwrite older ▼]                  │
   │  Filter: [________________________]                │
   │                                                    │
   │  Preserve: ☑ Timestamps  ☑ Attributes  ☐ ACL      │
   │  Symlinks: ◉ Copy link  ○ Follow  ○ Skip          │
   │                                                    │
   │  On finish: [Keep app open ▼]                      │
   │                                                    │
   │       [  Copy  ]  [  Add to Queue  ]  [  Cancel  ] │
   └────────────────────────────────────────────────────┘
```

4. Usuario presiona "Copy" → Transferencia inicia en segundo plano
5. El popup desaparece, aparece una **barra de transferencia minimizada** en la parte inferior:

```text
┌─ Left Panel ──────────────────┬─ Right Panel ─────────────────┐
│ ...                           │ ...                           │
│ ...                           │ ...                           │
│ ...                           │ ...                           │
├───────────────────────────────┴───────────────────────────────┤
│ 📋 Copying 3/10 files │ ████████░░░░ 45% │ 1.2 GB/s │ [▲][⏸]│
├───────────────────────────────────────────────────────────────┤
│ d:\GitHub\NCRust>                                             │
├───────────────────────────────────────────────────────────────┤
│ F1 Help  F2 Menu  F3 View  F4 Edit  F5 Copy ...              │
└───────────────────────────────────────────────────────────────┘
```

6. El usuario puede seguir navegando y trabajando normalmente
7. Puede presionar `[▲]` o la tecla asignada para **expandir** el panel de transferencia:

```text
┌────────────────────────────────────────────────────────────────┐
│  Transfer Engine                                     [▼ Min]  │
│ ──────────────────────────────────────────────────────────────│
│  Job 1: Copying to D:\Backup\          45%  1.2 GB/s  ETA 3m │
│  ████████████████████░░░░░░░░░░░░░░░░  3.5 GB / 7.8 GB       │
│  Current: DJI_0427.MP4 (176 MB)       5 / 10 files           │
│                                                               │
│  [ Pause ] [ Skip ] [ Stop ]        ☑ Unattended  ☑ Verify   │
│ ──────────────────────────────────────────────────────────────│
│  [File List] [Options] [Status] [Log]                         │
│                                                               │
│  ✓✓ D:\Videos\Drone\Waterfall\DJI_0424.MP4   AE9D  AE9D  942M│
│  ⚠  D:\Videos\Drone\Waterfall\DJI_0425.MP4   0706  2841  378M│
│     Hashes mismatch                                           │
│  ✓✓ D:\Videos\Drone\Waterfall\DJI_0426.MP4   48A5  48A5 1.4GB│
│  ▶  D:\Videos\Drone\Waterfall\DJI_0427.MP4        ████░ 1.1GB│
│  ·  D:\Videos\Drone\Waterfall\DJI_0428.MP4              469 M│
│  ·  D:\Videos\Drone\Waterfall\DJI_0429.MP4              539 M│
│ ──────────────────────────────────────────────────────────────│
│  Queue: 2 jobs pending                                        │
│   → Job 2: Copy D:\Data\Documents (1352 files, 1.0 GB)       │
│   → Job 3: Move E:\Temp\* → F:\Archive\ (89 files)           │
└────────────────────────────────────────────────────────────────┘
```

8. El usuario puede **minimizar** de nuevo con `[▼]` para volver a la barra compacta

#### Flujo: Encolamiento múltiple

```text
1. Usuario copia archivos (F5) → Job 1 inicia
2. Sin esperar, selecciona más archivos y presiona F5 → Job 2 se encola
3. Navega a otra carpeta, selecciona archivos, F6 (Move) → Job 3 se encola
4. Barra muestra: "Job 1: 45% │ Queue: 2 pending │ [▲][⏸]"
5. Al completar Job 1, automáticamente inicia Job 2
```

#### Flujo: Movimiento con verificación

```text
1. Usuario selecciona archivos, presiona F6 (Move)
2. Activa ☑ Verify
3. El motor ejecuta para cada archivo:
   a. Hash origen: SHA-256("file.mp4") → "abc123..."
   b. Copiar file.mp4 → destino/file.mp4
   c. Hash destino: SHA-256("destino/file.mp4") → "abc123..."
   d. Comparar: iguales → eliminar origen
4. Si hash no coincide: archivo marcado ⚠, origen NO eliminado
```

### 6.2. Cambios en el Layout

El layout actual tiene 4 bandas verticales:
```
Menu bar (0-1 líneas)
Main panels (flex)
CLI bar (1 línea)
F-keys bar (0-1 líneas)
```

Se agrega una **5ª banda condicional** entre los paneles y la CLI:

```
Menu bar (0-1 líneas)
Main panels (flex)
Transfer bar (0-2 líneas)    ← NUEVO (solo visible durante transferencias)
CLI bar (1 línea)
F-keys bar (0-1 líneas)
```

**Altura de la barra de transferencia:**
- **0 líneas:** Sin transferencias activas
- **1 línea:** Modo minimizado (barra compacta con progreso y velocidad)
- **2 líneas:** Modo minimizado extendido (+ nombre del archivo actual)

**Modo expandido:** Se renderiza como un popup overlay (similar a GitPanel o PluginMenu), no como una banda del layout. Esto permite mostrar toda la información sin robar espacio permanente a los paneles.

### 6.3. Mapeo de Teclas Nuevas

| Acción | Tecla sugerida | Descripción |
|--------|---------------|-------------|
| `ToggleTransferPanel` | `Ctrl+T` | Expandir/minimizar panel de transferencia |
| `PauseTransfer` | — | Pausar transferencia activa (desde popup expandido) |
| `ResumeTransfer` | — | Reanudar transferencia pausada |
| `CancelTransfer` | — | Cancelar transferencia activa |
| `SkipCurrentFile` | — | Saltar archivo actual |

> Las acciones Pause/Resume/Cancel/Skip se manejan como botones dentro del popup expandido, no como keybindings globales, para evitar conflictos con operaciones normales.

---

## 7. Plan de Implementación Detallado

El plan se divide en **8 secciones** independientes que pueden asignarse a sub-agentes paralelos. Cada sección tiene sub-secciones con detalle completo.

---

### Sección 1: Infraestructura Core — Tipos y Contratos

**Objetivo:** Definir todas las structs, enums, traits e interfaces que usarán todos los demás módulos.

**Archivos a crear:**
- `src/fs/transfer/mod.rs`
- `src/fs/transfer/job.rs`
- `src/fs/transfer/options.rs`
- `src/fs/transfer/events.rs`
- `src/fs/transfer/conflict.rs`
- `src/fs/transfer/filter.rs`

#### 1.1. `src/fs/transfer/mod.rs`

**Contenido:**
- Re-exportaciones públicas de todos los tipos
- Documentación del módulo con `//!` doc-comments

```rust
pub mod job;
pub mod options;
pub mod events;
pub mod conflict;
pub mod filter;
pub mod hash;
pub mod engine;
pub mod queue;
pub mod worker;
pub mod pipeline;
pub mod metadata;
pub mod report;
pub mod direct_io;
pub mod network;
pub mod post_action;
```

#### 1.2. `src/fs/transfer/job.rs`

**Contenido:**
- `TransferJob` — Estructura principal del trabajo
- `TransferOperation` — Enum Copy/Move
- `TransferJobStatus` — Máquina de estados
- `TransferProgress` — Progreso en tiempo real
- `TransferResults` — Resultados acumulados
- `FileTransferResult` — Resultado por archivo
- `FailedFile` — Archivo fallido
- `SkippedFile` — Archivo omitido

**Requisitos:**
- Todos los tipos deben derivar `Debug, Clone`
- `TransferJobStatus` debe implementar `Display` para la UI
- `TransferJob::new()` debe generar UUID automáticamente
- Incluir métodos helper: `is_active()`, `is_terminal()`, `elapsed()`

#### 1.3. `src/fs/transfer/options.rs`

**Contenido:**
- `TransferOptions` — Configuración de una transferencia
- `TransferOptionsBuilder` — Builder pattern
- `BufferSize` — Enum para tamaños de buffer (64KB, 256KB, 1MB, 4MB)
- `HashAlgorithm` — Enum para algoritmos de hash

**Requisitos:**
- Builder debe validar combinaciones inválidas (ej: direct_io con buffer < 4096)
- Serde Serialize/Deserialize para persistencia
- Default sensato para cada campo

#### 1.4. `src/fs/transfer/events.rs`

**Contenido:**
- `TransferEvent` — Eventos del worker hacia el main loop
- `TransferCommand` — Comandos del main loop hacia el worker

**Requisitos:**
- `TransferEvent` debe incluir `job_id` en cada variante
- `TransferCommand` debe ser Send + Sync

#### 1.5. `src/fs/transfer/conflict.rs`

**Contenido:**
- `ConflictResolution` — Enum de resoluciones
- `ConflictInfo` — Información del conflicto (src size/date vs dst size/date)
- `resolve_filename_conflict()` — Genera nombre alternativo `file (1).ext`

**Requisitos:**
- `resolve_filename_conflict()` debe manejar Unicode correctamente
- Debe iterar hasta encontrar un nombre disponible
- Test: verificar que `file.txt` → `file (1).txt` → `file (2).txt`

#### 1.6. `src/fs/transfer/filter.rs`

**Contenido:**
- `TransferFilter` — Struct con include/exclude patterns
- `FilterRule` — Enum con Glob, Size (min/max), Date (newer/older)
- `TransferFilter::matches()` — Verifica si un archivo pasa los filtros
- Parser para sintaxis: `"*.jpg;*.png"`, `">10MB"`, `"newer:30d"`

**Requisitos:**
- Reutilizar `glob_matches()` de `src/app/state/glob.rs`
- Parsear filtros desde una cadena unificada
- Test: verificar cada tipo de filtro

---

### Sección 2: Motor de Hash — Algoritmos de Verificación

**Objetivo:** Implementar todos los algoritmos de hash con interfaz unificada.

**Archivos a crear:**
- `src/fs/transfer/hash/mod.rs`
- `src/fs/transfer/hash/crc32.rs`
- `src/fs/transfer/hash/md5.rs`
- `src/fs/transfer/hash/sha1.rs`
- `src/fs/transfer/hash/sha256.rs`
- `src/fs/transfer/hash/blake3.rs`

#### 2.1. `src/fs/transfer/hash/mod.rs`

**Contenido:**
- Trait `HashStrategy`
- Factory function: `create_hasher(algorithm: HashAlgorithm) -> Box<dyn HashStrategy>`
- Re-exportaciones

**Trait `HashStrategy`:**
```rust
pub trait HashStrategy: Send + Sync {
    /// Nombre legible del algoritmo ("BLAKE3", "SHA-256", etc.)
    fn name(&self) -> &str;
    /// Alimentar datos al hasher (llamado múltiples veces)
    fn update(&mut self, data: &[u8]);
    /// Finalizar y producir el hash como string hexadecimal
    fn finalize(self: Box<Self>) -> String;
    /// Crear una nueva instancia limpia del mismo algoritmo
    fn new_instance(&self) -> Box<dyn HashStrategy>;
}
```

#### 2.2–2.6. Implementaciones individuales

Cada archivo implementa `HashStrategy` para su algoritmo:

- **CRC32** (`crc32fast`): `update()` delega a `crc32fast::Hasher`, `finalize()` retorna hex de 8 chars
- **MD5** (`md-5`): Usa `md5::Md5` del crate `md-5`, `finalize()` retorna hex de 32 chars
- **SHA-1** (`sha1`): Usa `sha1::Sha1`, `finalize()` retorna hex de 40 chars
- **SHA-256** (`sha2`): Usa `sha2::Sha256`, `finalize()` retorna hex de 64 chars
- **BLAKE3** (`blake3`): Usa `blake3::Hasher`, `finalize()` retorna hex de 64 chars

**Tests unitarios obligatorios para cada algoritmo:**
- Hash de cadena vacía
- Hash de "hello world"
- Hash de datos binarios grandes (1MB de zeros)
- Verificar que el hash coincide con valores conocidos de referencia

---

### Sección 3: Pipeline de Transferencia — Worker Core

**Objetivo:** Implementar el worker que ejecuta la transferencia archivo por archivo.

**Archivos a crear:**
- `src/fs/transfer/worker.rs`
- `src/fs/transfer/pipeline.rs`
- `src/fs/transfer/direct_io.rs`
- `src/fs/transfer/metadata.rs`

#### 3.1. `src/fs/transfer/worker.rs`

**Contenido:**
- `TransferWorker` — Struct principal del worker
- `TransferWorker::run()` — Función async principal

**Flujo de `run()`:**
1. **Fase 1 — Escaneo:** Recorrer todos los `sources` recursivamente, construir lista de `(src, dst)` con tamaños. Emitir `ScanProgress` periódicamente.
2. **Fase 2 — Preparación:** Crear directorios destino. Verificar espacio libre.
3. **Fase 3 — Transferencia:** Para cada archivo:
   a. Verificar si pasa los filtros
   b. Verificar conflictos de nombre
   c. Si hay conflicto, emitir `ConflictDetected` y esperar resolución
   d. Copiar archivo usando pipeline (ver 3.2)
   e. Preservar metadatos (ver 3.4)
   f. Emitir `FileCompleted` o `FileFailed`
4. **Fase 4 — Verificación (si activada):** Para cada archivo copiado, calcular hash destino y comparar
5. **Fase 5 — Limpieza (si move):** Eliminar archivos origen (solo los verificados si verify está activo)
6. **Fase 6 — Finalización:** Emitir `JobCompleted` con resultados

**Requisitos:**
- Verificar `is_paused` en cada iteración del loop de archivos
- Verificar `cancellation_token` en cada iteración
- Manejar errores por archivo (no abortar)
- Reintentar archivos fallidos según `max_retries`
- Yield to async runtime para mantener UI responsiva

#### 3.2. `src/fs/transfer/pipeline.rs`

**Contenido:**
- `PipelineCopier` — Implementa el pipeline de 3 etapas
- `PipelineCopier::copy_file()` — Copia un archivo con hash paralelo

**Cuando `verify = false`:** Pipeline simple de 2 etapas (read → write)
**Cuando `verify = true`:** Pipeline de 3 etapas (read → hash_src + write → hash_dst)

**Implementación del pipeline con verify:**
```text
tokio::spawn(reader_task) → mpsc(blocks) → tokio::spawn(writer_task)
                                         ↘ hash_updater(src)
                                           hash_updater(dst) ← writer callback
```

- Canal `mpsc` de bloques con backpressure (channel size = 4 bloques)
- Cada bloque: `Vec<u8>` de `buffer_size` bytes
- El reader lee del src y envía bloques
- El writer recibe bloques, escribe al dst, actualiza hash dst
- El hash src se actualiza en el reader antes de enviar

**Requisitos:**
- Fallback a copia secuencial si el pipeline falla
- Métricas de velocidad calculadas con ventana deslizante de 5 segundos
- Emitir `FileProgress` cada 100ms (throttled)

#### 3.3. `src/fs/transfer/direct_io.rs`

**Contenido:**
- `DirectIoReader` — Lectura sin caché del sistema
- `DirectIoWriter` — Escritura sin caché

**Requisitos:**
- Windows: `CreateFileW` con `FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH`
- Linux: `std::os::unix::fs::OpenOptionsExt` con `O_DIRECT`
- Alineación de buffers a sector size (4096 bytes)
- Manejo del último bloque parcial (no alineado)
- Fallback automático si el SO no soporta direct I/O

#### 3.4. `src/fs/transfer/metadata.rs`

**Contenido:**
- `MetadataPreservation` — Struct con flags de qué preservar
- `preserve_metadata(src, dst, options)` — Aplica metadatos del origen al destino

**Requisitos:**
- Timestamps: `filetime::set_file_mtime()` + `set_file_atime()` + platform-specific creation time
- Windows: `SetFileTime()` via `windows-sys` para creation time
- Atributos: `SetFileAttributesW()` en Windows
- Permisos: `std::fs::set_permissions()` en Unix
- ADS (Windows): Copiar alternate data streams si la opción está activa
- No fallar silenciosamente: loggear warnings si no se puede preservar un atributo

---

### Sección 4: Cola de Transferencias y Orquestador

**Objetivo:** Implementar la gestión de múltiples trabajos y el orquestador que los despacha.

**Archivos a crear:**
- `src/fs/transfer/queue.rs`
- `src/fs/transfer/engine.rs`
- `src/fs/transfer/post_action.rs`
- `src/fs/transfer/network.rs`

#### 4.1. `src/fs/transfer/queue.rs`

**Contenido:**
- `TransferQueue` — Cola thread-safe de trabajos

```rust
pub struct TransferQueue {
    jobs: Arc<Mutex<VecDeque<TransferJob>>>,
    active_job_id: Arc<Mutex<Option<Uuid>>>,
}
```

**Métodos:**
- `enqueue(job: TransferJob)` — Añadir trabajo al final
- `dequeue() -> Option<TransferJob>` — Sacar el siguiente trabajo
- `remove(job_id: Uuid) -> bool` — Eliminar trabajo pendiente
- `reorder(job_id: Uuid, direction: i32)` — Mover arriba/abajo
- `get_all() -> Vec<TransferJob>` — Snapshot de toda la cola
- `get_active() -> Option<TransferJob>` — Trabajo activo actual
- `pending_count() -> usize` — Trabajos pendientes
- `clear_completed()` — Limpiar trabajos terminados

#### 4.2. `src/fs/transfer/engine.rs`

**Contenido:**
- `TransferEngine` — Orquestador principal

```rust
pub struct TransferEngine {
    queue: TransferQueue,
    event_tx: mpsc::UnboundedSender<TransferEvent>,
    command_tx: Option<mpsc::Sender<TransferCommand>>,
    active_worker_handle: Option<JoinHandle<()>>,
    post_action: Option<PostAction>,
}
```

**Métodos públicos:**
- `new() -> (Self, UnboundedReceiver<TransferEvent>)` — Constructor + receptor de eventos
- `submit_job(job: TransferJob)` — Encolar y despachar si no hay activo
- `pause()` / `resume()` / `cancel()` / `skip_file()` — Comandos al worker activo
- `remove_from_queue(job_id: Uuid)` — Eliminar trabajo pendiente
- `set_post_action(action: PostAction)` — Configurar post-acción
- `get_queue_snapshot() -> Vec<TransferJob>` — Snapshot para la UI

**Lógica de despacho:**
- Cuando el worker activo completa (o falla/cancela), `process_next()` extrae el siguiente de la cola
- Si la cola queda vacía y hay `post_action`, ejecutar la post-acción

#### 4.3. `src/fs/transfer/post_action.rs`

**Contenido:**
- `PostAction` enum y ejecución

**Implementación de cada acción:**
- `Shutdown`: `shutdown /s /t 60` (Windows) o `shutdown -h +1` (Linux) — con aviso de 60s cancelable
- `EjectDrive`: Windows `CM_Request_Device_Eject` vía `windows-sys`; Linux `udisksctl power-off`
- `RunScript`: `tokio::process::Command::new(script_path).spawn()`
- Cada acción debe loggear su ejecución y resultado

#### 4.4. `src/fs/transfer/network.rs`

**Contenido:**
- `detect_network_drive(path: &Path) -> bool`
- `get_optimal_buffer_size(is_network: bool) -> usize`
- `NetworkConfig` — Timeout, reintentos, throttle

**Requisitos:**
- Windows: `GetDriveTypeW()` → `DRIVE_REMOTE`
- Linux: Parsear `/proc/mounts` buscando `cifs`, `nfs`, `sshfs`
- Integración con SSH existente (`src/fs/ssh.rs`): Si el panel tiene `ssh_conn`, usar el pipeline SSH

---

### Sección 5: Informes — Generación HTML/CSV

**Objetivo:** Generar informes exportables con resultados de transferencia.

**Archivos a crear:**
- `src/fs/transfer/report.rs`

#### 5.1. `src/fs/transfer/report.rs`

**Contenido:**
- `TransferReport` — Struct con datos del informe
- `generate_html(results: &TransferResults, options: &ReportOptions) -> String`
- `generate_csv(results: &TransferResults) -> String`
- `save_report(report: &str, format: ReportFormat, path: &Path) -> Result<()>`

**Template HTML:**
- Embebido con `include_str!("report_template.html")` o generado programáticamente
- Incluye tabla CSS-styled con filas coloreadas por estado (verde=ok, rojo=error, amarillo=warning)
- Resumen en la cabecera: totales, duración, velocidad media
- Responsive y legible en cualquier navegador

**Formato CSV:**
- Cabecera: `Source,Destination,Size,Status,SrcHash,DstHash,Error,Duration`
- UTF-8 con BOM para compatibilidad con Excel
- Escapar campos con comillas cuando contengan comas o saltos de línea

**Requisitos:**
- Path de reportes por defecto: `config::paths::get_cache_dir().join("reports")`
- Nombre del archivo: `transfer_report_YYYYMMDD_HHMMSS.{html,csv}`
- No generar automáticamente por defecto (solo bajo demanda o si la opción está activa)

---

### Sección 6: Estado de Aplicación — Integración con AppState

**Objetivo:** Conectar el TransferEngine con el event loop de Pairee.

**Archivos a crear:**
- `src/app/state/transfer_state.rs`

**Archivos a modificar:**
- `src/app/state/mod.rs`
- `src/app/state/types.rs`
- `src/app/app/background.rs`
- `src/keybindings/actions.rs`
- `src/config/settings.rs`
- `src/config/localization/en.rs`

#### 6.1. `src/app/state/transfer_state.rs`

**Contenido:**
- `TransferUIState` — Estado de UI del motor de transferencia

```rust
pub struct TransferUIState {
    /// Motor de transferencia
    pub engine: TransferEngine,
    /// Receptor de eventos del motor
    pub event_rx: mpsc::UnboundedReceiver<TransferEvent>,
    /// Modo de visualización actual
    pub view_mode: TransferViewMode,
    /// Pestaña activa en el popup expandido
    pub active_tab: usize,
    /// Cursor en la lista de archivos
    pub file_list_cursor: usize,
    /// Scroll de la lista de archivos
    pub file_list_scroll: usize,
    /// Cursor en la cola
    pub queue_cursor: usize,
    /// Snapshot del progreso actual (para rendering)
    pub current_progress: Option<TransferProgress>,
    /// Snapshot de resultados del trabajo activo
    pub current_results: Option<TransferResults>,
    /// Jobs completados (para historial de la sesión)
    pub completed_jobs: Vec<TransferJob>,
}

pub enum TransferViewMode {
    Hidden,        // Sin transferencias activas
    Minimized,     // Barra compacta (1-2 líneas)
    Expanded,      // Popup completo
}
```

#### 6.2. Modificación de `src/app/state/mod.rs`

**Cambios:**
- Añadir campo `pub transfer: Option<TransferUIState>` al `AppState`
- Inicializar como `None` en `AppState::new()`
- Crear `TransferUIState` lazy (al primer F5/F6) para no consumir recursos si no se usa

#### 6.3. Modificación de `src/app/app/background.rs`

**Cambios:**
- Añadir nueva sección `// 1.9 Process Transfer Engine events` en `process_background_updates()`
- Drenar `transfer.event_rx` en cada frame
- Actualizar `transfer.current_progress` y `transfer.current_results` según los eventos
- Cuando llega `JobCompleted`: mover job a `completed_jobs`, despachar siguiente
- Cuando llega `ConflictDetected`: mostrar popup de resolución de conflicto

**Compatibilidad backward:**
- El sistema antiguo (`progress_rx`, `CopyProgress`, `BackgroundOpContext`) se mantiene temporalmente
- Nueva flag `use_transfer_engine: bool` en Settings decide cuál usar
- En releases futuras, eliminar el sistema antiguo

#### 6.4. Modificación de `src/keybindings/actions.rs`

**Nuevas acciones:**
```rust
/// Toggle transfer panel expand/minimize (Ctrl+T)
ToggleTransferPanel,
```

#### 6.5. Modificación de `src/config/settings.rs`

**Nuevos campos en `Settings`:**
```rust
// ── Transfer Engine settings ─────────────────────────────────
#[serde(default = "default_true")]
pub transfer_engine_enabled: bool,
#[serde(default = "default_transfer_hash")]
pub transfer_default_hash: String,         // "blake3"
#[serde(default = "default_transfer_buffer")]
pub transfer_buffer_size: u32,             // 1048576 (1MB)
#[serde(default)]
pub transfer_verify_after_copy: bool,      // false
#[serde(default)]
pub transfer_direct_io: bool,              // false
#[serde(default)]
pub transfer_preserve_timestamps: bool,    // true
#[serde(default)]
pub transfer_preserve_attributes: bool,    // true
#[serde(default)]
pub transfer_max_retries: u32,             // 3
#[serde(default = "default_transfer_conflict")]
pub transfer_conflict_resolution: String,  // "ask"
#[serde(default)]
pub transfer_skip_symlinks: bool,          // false
#[serde(default)]
pub transfer_auto_report: bool,            // false
#[serde(default = "default_transfer_report_format")]
pub transfer_report_format: String,        // "html"
```

#### 6.6. Modificación de `src/config/localization/en.rs`

**Nuevas claves de traducción (mínimo):**
```rust
// Transfer Engine
("transfer_title", "Transfer Engine"),
("transfer_copying", "Copying"),
("transfer_moving", "Moving"),
("transfer_paused", "Paused"),
("transfer_completed", "Completed"),
("transfer_failed", "Failed"),
("transfer_cancelled", "Cancelled"),
("transfer_scanning", "Scanning files..."),
("transfer_verifying", "Verifying integrity..."),
("transfer_queue", "Queue"),
("transfer_file_list", "File List"),
("transfer_options", "Options"),
("transfer_status", "Status"),
("transfer_log", "Log"),
("transfer_pause", "Pause"),
("transfer_resume", "Resume"),
("transfer_skip", "Skip"),
("transfer_stop", "Stop"),
("transfer_retry_failed", "Retry Failed"),
("transfer_export_report", "Export Report"),
("transfer_hash_mismatch", "Hashes mismatch"),
("transfer_hash_match", "Verified OK"),
("transfer_speed", "{}/s"),
("transfer_eta", "ETA {}"),
("transfer_files_progress", "{} / {} files"),
("transfer_bytes_progress", "{} / {}"),
("transfer_job_pending", "{} jobs pending"),
("transfer_on_finish", "On finish"),
("transfer_keep_open", "Keep app open"),
("transfer_conflict_title", "File already exists"),
("transfer_conflict_overwrite", "Overwrite"),
("transfer_conflict_skip", "Skip"),
("transfer_conflict_rename", "Rename"),
("transfer_conflict_overwrite_older", "Overwrite if older"),
("transfer_conflict_apply_all", "Apply to all"),
("transfer_direct_io", "Direct I/O (bypass cache)"),
("transfer_preserve_timestamps", "Preserve timestamps"),
("transfer_preserve_attributes", "Preserve attributes"),
("transfer_verify_after", "Verify files after transfer"),
("transfer_buffer_size", "Buffer size"),
("transfer_hash_algorithm", "Hash algorithm"),
("transfer_filter", "Filter"),
("transfer_add_to_queue", "Add to Queue"),
("transfer_error_read", "Error reading file: {}"),
("transfer_error_write", "Error writing file: {}"),
("transfer_error_disk_full", "Destination disk is full"),
("transfer_error_permission", "Permission denied: {}"),
("transfer_retrying", "Retrying ({}/{})..."),
```

> **Nota:** Seguir el skill `localize-helper` para agregar estas claves correctamente.

---

### Sección 7: Renderizado UI — Componentes de Ratatui

**Objetivo:** Implementar todos los componentes visuales del Transfer Engine.

**Archivos a crear:**
- `src/ui/transfer/mod.rs`
- `src/ui/transfer/bar.rs`
- `src/ui/transfer/panel.rs`
- `src/ui/transfer/queue_view.rs`
- `src/ui/transfer/file_list.rs`
- `src/ui/transfer/options_view.rs`

**Archivos a modificar:**
- `src/ui/mod.rs`
- `src/ui/layout.rs`
- `src/ui/popup/mod.rs`

#### 7.1. `src/ui/transfer/bar.rs` — Barra Minimizada

**Renderiza la barra compacta de 1-2 líneas en la parte inferior:**

```text
📋 Copying 3/10 files │ ████████░░░░ 45% │ 1.2 GB/s │ ETA 2m │ [▲][⏸]
```

**Componentes Ratatui:**
- `Gauge` para la barra de progreso
- `Paragraph` para texto de estado
- Layout horizontal con `Constraint::Min` y `Constraint::Length`

**Requisitos:**
- Color: Verde para progreso OK, amarillo para pausado, rojo si hay errores
- Click en `[▲]` expande el popup
- Click en `[⏸]` pausa/reanuda
- Si hay múltiples jobs: `"Job 1/3: Copying..."` con indicador de cola

#### 7.2. `src/ui/transfer/panel.rs` — Panel Expandido

**Renderiza el popup overlay completo (similar a GitPanel o PluginMenu):**

**Estructura:**
```text
┌─ Transfer Engine ─────────────────────────── [▼ Minimize] ──┐
│  HEADER: Progreso general + velocidad + ETA                  │
│  TABS: [File List] [Options] [Status] [Log]                  │
│  CONTENT: Contenido de la pestaña activa                     │
│  FOOTER: Botones [Pause] [Skip] [Stop] + opciones            │
│  QUEUE: Vista compacta de trabajos en cola                    │
└──────────────────────────────────────────────────────────────┘
```

**Requisitos:**
- Tamaño: 80% ancho × 70% alto (usando `centered_rect()` existente)
- Tabs navegables con ← → o Tab
- Tecla Esc minimiza (no cierra, porque la transferencia sigue)
- F-keys del popup: no conflictos con F-keys globales

#### 7.3. `src/ui/transfer/file_list.rs` — Lista de Archivos

**Renderiza la tabla de archivos (similar a TeraCopy):**

```text
Status │ Source Path                          │ Src Hash  │ Dst Hash  │ Size
───────┼──────────────────────────────────────┼───────────┼───────────┼──────
  ✓✓   │ D:\Videos\Drone\DJI_0424.MP4        │ AE9D:90F6 │ AE9D:90F6 │ 942M
  ⚠    │ D:\Videos\Drone\DJI_0425.MP4        │ 0706:6BDB │ 2841:D4EA │ 378M
       │   Hashes mismatch                    │           │           │
  ✓✓   │ D:\Videos\Drone\DJI_0426.MP4        │ 48A5:B891 │ 48A5:B891 │ 1.4G
  ▶    │ D:\Videos\Drone\DJI_0427.MP4        │           │ ████░░░░  │ 1.1G
  ·    │ D:\Videos\Drone\DJI_0428.MP4        │ 26A3:CA50 │           │ 469M
```

**Componentes Ratatui:**
- `Table` widget con columnas configurables
- Colores por estado: verde (✓✓), rojo (✗), amarillo (⚠), azul (▶ activo), gris (· pendiente)
- Scrollable con PageUp/PageDown
- Hash truncado a 4+4 bytes para legibilidad (ej: `AE9D:90F6`)

#### 7.4. `src/ui/transfer/queue_view.rs` — Vista de Cola

**Renderiza la lista de trabajos en cola:**

```text
Queue (2 pending):
  → Job 2: Copy D:\Data\Documents (1352 files, 1.0 GB)
  → Job 3: Move E:\Temp\* → F:\Archive\ (89 files, 340 MB)
```

**Requisitos:**
- Seleccionable con cursor para eliminar/reordenar
- Shortcuts: Del para eliminar, +/- para reordenar

#### 7.5. `src/ui/transfer/options_view.rs` — Vista de Opciones

**Renderiza la pestaña de opciones de transferencia activa (similar a TeraCopy Status tab):**

```text
Transfer:                  Transfer options:         Hash:
  ☑ Files and folders        [Overwrite all ▼]        [BLAKE3 ▼]
    ☑ Data                   ☐ Export as HTML          [1.0 MB ▼] buffer
    ☑ Timestamps             ☑ Verify after copy
    ☑ Attributes             ☐ Save checksum file    On finish:
    ☐ Streams                                          [Keep app open ▼]
    ☐ Security/ACL
```

#### 7.6. Modificación de `src/ui/layout.rs`

**Cambios en `calculate_layout()`:**
- Añadir nuevo `Rect` para la barra de transferencia: `pub transfer_bar_rect: Rect`
- La altura depende de `TransferViewMode`:
  - `Hidden` → `Constraint::Length(0)`
  - `Minimized` → `Constraint::Length(1)` o `Length(2)`
- Insertarla entre `main_rect` (paneles) y `cli_rect`

#### 7.7. Modificación de `src/ui/mod.rs`

**Cambios en `draw_ui()`:**
- Después de renderizar los paneles, renderizar `transfer::bar::render_transfer_bar()`
- Si `TransferViewMode::Expanded`, renderizar `transfer::panel::render_transfer_panel()` como overlay

#### 7.8. Modificación de `src/ui/popup/mod.rs`

**Cambios:**
- Añadir `pub mod transfer;` (o integrar con el módulo `ui::transfer`)
- Nuevo `PopupType::TransferPanel { ... }` en `types.rs` (o usar el sistema independiente)

---

### Sección 8: Integración de Input y Handlers

**Objetivo:** Conectar las acciones de teclado y los handlers de popup con el TransferEngine.

**Archivos a modificar:**
- `src/app/input.rs`
- `src/app/input_popup/mod.rs`
- `src/app/input_popup/copy.rs`
- `src/app/input_popup/rename_move.rs`
- `src/app/input_popup/confirm_dialogs.rs`
- `src/app/actions/fs_ops/copy.rs`
- `src/app/actions/fs_ops/move_rename.rs`
- `src/keybindings/resolver.rs`

#### 8.1. Nuevo handler: `src/app/input_popup/transfer_panel.rs`

**Contenido:**
- Handler de input para el popup expandido de transferencia
- Manejo de tabs (← →), scroll (↑ ↓ PgUp PgDn), botones (Enter, P, S, Esc)

**Teclas del popup expandido:**
| Tecla | Acción |
|-------|--------|
| `Esc` | Minimizar popup |
| `Tab` | Siguiente pestaña |
| `Shift+Tab` | Pestaña anterior |
| `↑` / `↓` | Navegar lista de archivos |
| `PgUp` / `PgDn` | Scroll rápido |
| `p` | Pause/Resume |
| `s` | Skip archivo actual |
| `x` | Stop (cancelar trabajo) |
| `r` | Retry failed files |
| `e` | Export report |
| `Del` | Eliminar job de cola (si cursor en Queue) |
| `+` / `-` | Reordenar cola |

#### 8.2. Modificación de handlers de F5/F6

**Cambio principal:**
- Cuando `settings.transfer_engine_enabled == true`:
  - F5 crea un `TransferJob` con `operation: Copy` y lo envía a `TransferEngine::submit_job()`
  - F6 crea un `TransferJob` con `operation: Move` y lo envía a `TransferEngine::submit_job()`
  - NO se crea `CopyProgress` popup (se usa la barra de transferencia)
- Cuando `settings.transfer_engine_enabled == false`:
  - Comportamiento legacy actual (para backward compatibility)

#### 8.3. Modificación de `src/app/input.rs`

**Cambios:**
- Mapear `Action::ToggleTransferPanel` a expandir/minimizar el panel de transferencia
- Si no hay transferencias activas, mostrar Info popup "No active transfers"

#### 8.4. Modificación de `src/keybindings/resolver.rs`

**Cambios:**
- Mapear `Ctrl+T` a `Action::ToggleTransferPanel` en todos los presets
- Registrar en los archivos de preset (`keymaps/*.toml`)

---

## Apéndice A: Dependencias Cargo.toml a Agregar

```toml
# Hash algorithms for transfer verification
blake3 = "1"
md-5 = "0.10"
sha1 = "0.10"
sha2 = "0.10"
crc32fast = "1"

# Utility
uuid = { version = "1", features = ["v4"] }
filetime = "0.2"
bytesize = "1"
```

## Apéndice B: Orden de Implementación Recomendado

```text
Fase 1 (Semana 1-2): Sección 1 (Tipos) + Sección 2 (Hash)
    → Paralelizable: Son independientes entre sí
    → Resultado: Tipos compilables + hash testeado

Fase 2 (Semana 2-3): Sección 3 (Worker/Pipeline)
    → Depende de: Sección 1, Sección 2
    → Resultado: Worker funcional sin UI

Fase 3 (Semana 3-4): Sección 4 (Cola/Engine)
    → Depende de: Sección 3
    → Resultado: Motor completo sin UI

Fase 4 (Semana 4-5): Sección 5 (Informes) + Sección 6 (Estado)
    → Paralelizable: Sección 5 es independiente, Sección 6 depende de S1+S4
    → Resultado: Motor integrado con AppState

Fase 5 (Semana 5-7): Sección 7 (UI) + Sección 8 (Input)
    → Depende de: Sección 6
    → Resultado: Feature completa end-to-end

Fase 6 (Semana 7-8): Testing de integración + Polish
    → Verificar todos los flujos
    → Ajustar UX basado en uso real
```

## Apéndice C: Matriz de Compatibilidad

| Feature | Windows | Linux | macOS | SSH Remote |
|---------|---------|-------|-------|------------|
| Copia básica | ✓ | ✓ | ✓ | ✓ |
| Direct I/O | ✓ (`FILE_FLAG_NO_BUFFERING`) | ✓ (`O_DIRECT`) | ✗ (fallback) | ✗ |
| Hash verification | ✓ | ✓ | ✓ | ✓ |
| Preserve timestamps | ✓ | ✓ | ✓ | Parcial |
| Preserve attributes | ✓ (Windows attrs) | ✓ (Unix perms) | ✓ | Parcial |
| ADS streams | ✓ | ✗ (N/A) | ✗ (N/A) | ✗ |
| Long paths | ✓ (`\\?\` prefix) | N/A | N/A | N/A |
| Network detection | ✓ (`GetDriveTypeW`) | ✓ (`/proc/mounts`) | ✓ | ✓ |
| Post-actions | ✓ | ✓ | ✓ | ✗ |
| Symlink handling | ✓ | ✓ | ✓ | Parcial |

## Apéndice D: Migración del Sistema Antiguo

### Plan de migración gradual:

1. **v0.7.0:** Ambos sistemas coexisten. `transfer_engine_enabled = true` por defecto, pero el usuario puede desactivarlo.
2. **v0.8.0:** El sistema antiguo se marca como `deprecated` con warnings en el log.
3. **v0.9.0:** Se elimina el sistema antiguo (`fs::ops_worker/copy.rs`, `copy_move.rs`, `move_rename.rs`) y la flag `transfer_engine_enabled`.

### Archivos legacy a eliminar eventualmente:
- `src/fs/ops_worker/copy.rs` → Reemplazado por `src/fs/transfer/worker.rs`
- `src/fs/ops_worker/copy_move.rs` → Reemplazado por `src/fs/transfer/engine.rs`
- `src/fs/ops_worker/move_rename.rs` → Reemplazado por `src/fs/transfer/worker.rs`
- `PopupType::CopyProgress` → Reemplazado por `TransferUIState`
- `BackgroundOpContext` → Reemplazado por `TransferJob`
- `AppState::progress_rx` → Reemplazado por `TransferUIState::event_rx`
- `AppState::active_bg_op` → Reemplazado por `TransferEngine::active_job`

> **Importante:** Los archivos `compress.rs`, `extract.rs`, y `wipe.rs` dentro de `ops_worker` NO se eliminan — son operaciones diferentes que no son parte del Transfer Engine.
