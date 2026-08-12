# Pairee — Plan de Mejora y Seguimiento

> **Versión base:** 0.7.2  
> **Fecha de inicio:** 2026-08-12  
> **Rama de trabajo:** `master`  
> **Objetivo:** elevar calidad, arquitectura, CI, tests y DX sin archivos monolíticos, usando patrones de diseño modernos.

Este documento es la **fuente de verdad del progreso**. Cada ítem usa checkbox:

- `[ ]` pendiente  
- `[x]` completado (con commit referenciado cuando aplique)

Actualizar este archivo **entre tarea y tarea**, junto con commit + push.

---

## 1. Resumen ejecutivo

**Pairee** es un gestor de archivos dual-panel en Rust (estilo Norton/Far), con TUI (`ratatui`/`crossterm`), operaciones async, motor de transferencias, Git, SSH/SFTP, plugins Lua, auto-update y empaquetado multiplataforma.

### Indicadores (baseline → actual)

| Indicador | Baseline (2026-08-12) | Actual |
|-----------|----------------------|--------|
| Fuentes Rust | ~316 archivos, ~44 700 LOC | sin re-conteo global |
| Tests | 115 unitarios; `tests/` vacío | **115 unit + 2 integration** |
| Binario release | ~15.6 MB | sin cambio de features |
| Idiomas UI | EN + ES | sin cambio |
| Rama default | `master` | CI alineado a `master`/`main` |
| Clippy crate-level | `#![allow(clippy::all)]` | **eliminado**; `-D warnings` OK |
| CI check branches | solo `main` (incorrecto) | **`master` + `main` + matrix OS** |

### Prioridades

| P | Tema | Estado |
|---|------|--------|
| P0 | CI en `master` + matrix multi-OS | **Hecho** |
| P0 | Quitar `allow(clippy::all)` + tool configs | **Hecho** |
| P0 | README / MSRV / docs status | **Hecho** |
| P0 | Limpieza de archivos obsoletos | **Hecho** |
| P1 | Unificar transferencias (legacy vs engine) | Pendiente |
| P1 | Partir God objects (`AppState`, `PopupType`) | Pendiente (allow temporal en enum) |
| P1 | Tests de integración + cobertura | En curso (2 tests base) |
| P2 | Roadmap plugins (G1–G14) | Pendiente |
| P2 | Sincronizar design docs con código real | Parcial (banners + índice) |
| P3 | Feature flags, binario, i18n, command palette | Pendiente |

---

## 2. Principios de diseño (obligatorios en este plan)

Aplicar en **todo** refactor nuevo:

| Principio / patrón | Aplicación en Pairee |
|--------------------|----------------------|
| **SRP + módulos finos** | Un archivo = una responsabilidad. Prohibido crecer >500 LOC; partir en directorio. |
| **Separation of Concerns** | Core (`fs`, `config`, domain) sin `ratatui`. UI solo lee estado y emite acciones. |
| **Command** | `Action` / handlers por comando; sin lógica de negocio en widgets. |
| **Strategy** | Backends de transferencia (local, SSH), presets de keybindings, hash algorithms. |
| **Facade** | `TransferEngine`, `PluginManager` como fachadas estables. |
| **Observer / Event bus** | Unificar canales `Option<Receiver>` en eventos tipados. |
| **State / Dialog stack** | Reemplazar mega-enum `PopupType` por stack de diálogos tipados. |
| **Repository** | Persistencia (settings, history, plugins.lock) detrás de APIs claras. |
| **Builder** | Jobs de transfer, opciones de copia, manifests de plugin. |
| **Ports & Adapters** | FS/SSH/Git como puertos; implementaciones intercambiables. |
| **Fail-safe defaults** | `Result` + logging; sin `unwrap` en paths de usuario. |

Referencias internas: [`.agents/AGENTS.md`](../.agents/AGENTS.md), skill `rust-best-practices`.

---

## 3. Fortalezas a preservar

- [x] Producto diferenciado (dual-panel + Rust + plugins + Git + SSH + transfer)
- [x] Modularización parcial (`fs_ops/`, `transfer/`, `plugin/runtime/bindings/`)
- [x] Auditorías de seguridad recientes
- [x] Sandbox de plugins + secure mode
- [x] i18n centralizada EN/ES
- [x] Distribución (installers, deb/rpm, winget, MSIX, SHA-256)
- [x] Skills de agentes (changelog, settings, localize)

---

## 4. Fase A — Higiene P0 (máximo ROI)

### A.1 Plan de seguimiento y docs de control

- [x] Crear `docs/IMPROVEMENT_PLAN.md` (este archivo)
- [x] Entrada en `docs/UNRELEASED.md` por cambios user-facing / DX relevantes

### A.2 CI/CD

- [x] `check.yml`: triggers en `master` (y `main` por compat)
- [x] Matrix de tests: `ubuntu-latest` + `windows-latest`
- [x] Alinear actions a Node 24 (`checkout@v7`, `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24`)
- [x] Job policy: fallar si reaparece `#![allow(clippy::all)]` en `src/main.rs`
- [ ] (Opcional) `cargo deny` / audit de dependencias

### A.3 Clippy / formato / MSRV

- [x] Eliminar `#![allow(clippy::all)]` de `src/main.rs`
- [x] Añadir `rustfmt.toml` y `clippy.toml` (sin monolitos de config)
- [x] Declarar `rust-version` (MSRV `1.85`) en `Cargo.toml`
- [x] Corregir lints hasta `cargo clippy --all-targets -- -D warnings`
- [x] Actualizar README (MSRV, badge 1.85+)

### A.4 Documentación alineada a la realidad

- [x] Arreglar enlaces rotos en `README.md` / `README.es.md` (`help/en/...`, `help/es/...`)
- [x] Actualizar árbol de estructura del proyecto en README
- [x] Reemplazar `docs/README.md` por **índice de docs con estado**
- [x] Banner de estado en `plugin-system-design.md` (Partial / Implemented)
- [x] Banner de estado en `transfer-engine-design.md` (Implemented / Partial)
- [x] README: plugins ya no “solo Planned”

### A.5 Limpieza de archivos obsoletos

- [x] Eliminar directorios temporales `.tmp*` del workspace
- [x] Reemplazar `.gitignore` heredado de rustc por uno propio de Pairee
- [x] Eliminar `example/reference` local (referencia yazi; gitignored)
- [x] No borrar assets de producto (`descript.ion`, manifests, help)

### A.6 Tests mínimos de red

- [x] Primer test de integración en `tests/` (settings TOML + temp isolation)
- [x] Verificar CONTRIBUTING (fmt/clippy/test) sigue válido

**Fase A: completa salvo `cargo deny` opcional.**

---

## 5. Fase B — Unificación del Transfer Engine (P1) ⬅ siguiente

**Patrones:** Strategy (backend Local/Ssh), Facade (`TransferEngine`), Builder (`TransferJob`/`TransferOptions`), Observer (eventos de progreso).

- [ ] Inventario de call sites: `ops_worker` vs `transfer`
- [ ] Diseñar trait `TransferBackend` (local + SSH) en módulos finos (no un solo archivo)
- [ ] Migrar copy/move/delete SSH al engine
- [ ] Deprecar `spawn_copy_move_task` path dual en UI
- [ ] Un solo modelo de progreso/UI
- [ ] Tests: conflicto, cancel, verify hash, move tree
- [ ] Partir `fs/transfer/worker.rs` (>1000 LOC) en módulos (`scan`, `copy_file`, `verify`, `ssh_ops`, …)

---

## 6. Fase C — Estado y modularidad (P1)

**Patrones:** State objects, Dialog stack, Facade de servicios, Event bus.

- [ ] Extraer de `AppState`: `TransferUiState`, `UpdateState`, `PluginHostState`, `HistoryState`, `PanelPair`
- [ ] Reemplazar mega-enum `PopupType` por stack de diálogos tipados (un módulo por familia)
  - Nota: hoy tiene `#[allow(clippy::large_enum_variant)]` temporal
- [ ] Partir archivos >500 LOC (lista baseline abajo)
- [ ] Sustituir poll `take/unwrap/put-back` de receivers por bus o `select!` idiomático
- [ ] Valorar `src/lib.rs` + binario fino para tests/benches

### Archivos monolíticos a partir (baseline)

| Archivo | ~LOC | Estado |
|---------|------|--------|
| `src/fs/transfer/worker.rs` | ~1037 | [ ] |
| `src/app/input_popup/plugin_menu/dev/options.rs` | ~793 | [ ] |
| `src/app/actions/ui_settings.rs` | ~766 | [ ] |
| `src/ui/popup/history_lists.rs` | ~742 | [ ] |
| `src/ui/transfer/panel.rs` | ~676 | [ ] |
| `src/keybindings/preset.rs` | ~602 | [ ] |
| `src/app/state/types.rs` | ~598 | [ ] |
| `src/plugin/updater.rs` | ~552 | [ ] |
| `src/fs/list.rs` | ~521 | [ ] |
| `src/config/settings.rs` | ~518 | [ ] |

---

## 7. Fase D — Plugins productivos (P2)

Basado en `docs/technical/plugin-roadmap.md` (G1–G14).

- [ ] Diálogos reales end-to-end (confirm/input/select)
- [ ] Userdata tipados (`File`, metadata, mime)
- [ ] API async FS + `Command` builder con streaming
- [ ] Contexto vivo `cx` (no solo snapshot)
- [ ] Plugins de aceptación en CI
- [ ] API docs versionadas (semver de superficie Lua)
- [ ] Actualizar README + help de plugins

---

## 8. Fase E — Producto y distribución (P3)

- [ ] Command palette sobre `Action`
- [ ] Onboarding / primer arranque (elegir preset de teclas)
- [ ] Más idiomas (pipeline localize-helper)
- [ ] Feature flags (`ssh`, `git`, `plugins`, `image-preview`)
- [ ] CI macOS si se declara soporte oficial
- [ ] Threat model corto (plugins, SSH, update, elevated helper)
- [ ] Fuzzing parsers (config TOML, manifests, descript.ion, globs)

---

## 9. Métricas objetivo (3 meses)

| Métrica | Baseline | Objetivo | Actual |
|---------|----------|----------|--------|
| CI en rama default | No | Sí | **Sí (`master`/`main`)** |
| Platforms en CI | Linux (mal cableado) | Linux + Windows | **Sí** |
| Clippy crate allow all | Sí | No | **No** |
| Tests | 115 unit | 115+ y ≥15 integration | **115 + 2** |
| Archivos >800 LOC | ≥2 | 0 | ≥2 (pendiente Fase C) |
| Docs con status real | Desfasadas | Índice OK | **Índice + banners** |
| Gaps plugins P0 | Abiertos | Diálogos + 1 ejemplo E2E | Abiertos |

---

## 10. Registro de commits de este plan

| Fecha | Commit | Qué se hizo |
|-------|--------|-------------|
| 2026-08-12 | _(serie de commits en esta sesión)_ | Fase A: plan, CI, clippy real, docs, limpieza, tests integración |

Ver historial git de `master` con mensajes `docs:`, `ci:`, `chore:`, `refactor:`, `test:`.

---

## 11. Roadmap visual

```text
Alto impacto │  [x CI master] [x Clippy real] [ Unificar transfer ]
             │  [ Partir AppState/PopupType ] [ Tests integración+ ]
             │  [ Plugins: diálogos + userdata ]
             │  [x Docs sync base] [ Feature flags ]
Bajo impacto │  [ Más idiomas ] [ Command palette ] [ macOS CI ]
             └────────────────────────────────────────────
               Bajo esfuerzo              Alto esfuerzo
```

---

## 12. Conclusión operativa

**Fase A (higiene) cerrada.**  
Siguiente bloque de trabajo recomendado: **Fase B — unificar Transfer Engine** partiendo `worker.rs` en módulos Strategy/Facade, sin path dual `ops_worker` vs `transfer`.

---

*Última actualización del progreso: 2026-08-12 (Fase A completada localmente; push pendiente en la serie de commits).*
