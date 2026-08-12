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
| P0 | CI / Clippy / docs / limpieza | **Hecho** (Fase A) |
| P1 | Transfer Engine unificado + sin legacy progress | **Hecho** (Fase B) |
| **P1** | **Input (`keybinds` / which-key) + scrollbars + anti-glitch TUI** | **Siguiente (Fase F)** |
| P1 | Partir God objects (`AppState`, `PopupType`) | Pendiente (Fase C) |
| P1 | Tests de integración + cobertura | En curso |
| P2 | Roadmap plugins (G1–G14) | Pendiente (Fase D) |
| P3 | Feature flags, i18n extra, onboarding | Parcial (palette hecho) |

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

## 5. Fase B — Unificación del Transfer Engine (P1) ✅ cerrada

**Patrones:** Strategy (`backend/local` + `backend/ssh` + `backend/ops_jobs`), Facade (`submit_simple` / `submit_apply_command`), Builder (`options_from_settings`), Observer (`TransferEvent`), cooperative cancel (`AtomicBool` + `ensure_not_cancelled`).

**Decisión de diseño:** copy/move/delete/wipe/compress/extract/**apply-command** comparten **una** UI de trabajos (cola, minimizar, log, cancel). Beta: se eliminó el stack legacy (`ops_worker` spawns, `progress_rx`, modal `CopyProgress`).

- [x] Backends Strategy: local, SSH, ops_jobs (wipe/compress/extract/**ApplyCommand**)
- [x] UI unificada Transfer Engine (sin modal de progreso paralelo)
- [x] **Apply command** en engine (`shell_template` + `%f`)
- [x] **Cancel mid-archive** cooperativo en zip/tar/7z nativo + kill del 7z externo
- [x] Eliminado `src/fs/ops_worker/`, `apply_cmd` spawn, `CopyProgress`, `progress_rx`, `BackgroundOpContext`
- [x] Tests + clippy verdes
- [x] SSH wipe/compress/extract/apply: no soportados (explícito)

### 5.1 Inventario de paths

| Capacidad | Path | UI |
|-----------|------|-----|
| Copy/Move/Delete local+SSH | `backend::{local,ssh}` | Transfer panel |
| Wipe / Compress / Extract / Apply | `backend::ops_jobs` | Transfer panel |

### 5.2 Layout post-split de `worker/`

| Archivo | Rol | ~LOC |
|---------|-----|------|
| `worker/mod.rs` | Facade: `TransferWorker::run` | ~129 |
| `worker/scan.rs` | Fase escaneo → `ScanOutcome` | ~163 |
| `worker/delete_phase.rs` | Delete / recycle | ~205 |
| `worker/copy_phase.rs` | Copy/move, conflictos, verify | ~460 |
| `worker/destination.rs` | `is_destination_parent_dir` | ~59 |
| `worker/fs_helpers.rs` | recycle bin + make writable | ~116 |
| `worker/speed.rs` | reporter de velocidad (DRY) | ~43 |
| `worker/tests.rs` | test move tree | ~54 |

---

## 6. Fase F — Input moderno, scrollbars y estabilidad TUI (P1) ⬅ **siguiente**

> Objetivo: simplificar atajos, mejorar discoverability, scrollbars correctos y eliminar glitches gráficos en Windows/Linux.  
> Referencias de estudio: crates `keybinds`, `ratatui-which-key`, `tui-scrollbar`; y patrones del repo [xai-org/grok-build](https://github.com/xai-org/grok-build) (clon temporal de análisis, no se vendoría en Pairee).

### 6.1 Decisión: `keybinds` vs `ratatui-which-key` (juntos o uno)

| Criterio | [`keybinds`](https://crates.io/crates/keybinds) | [`ratatui-which-key`](https://crates.io/crates/ratatui-which-key) |
|----------|--------------------------------------------------|-------------------------------------------------------------------|
| Rol | Dispatcher/parser/generator de atajos (agnóstico de UI) | Keymap + routing por **scope** + popup estilo Neovim which-key |
| Config TOML/serde | Sí (feature `serde`) | API de builder en código; strings tipo `"<c-c>"`, secuencias |
| Secuencias multi-tecla | Sí (`Ctrl+x Ctrl+s`, timeout) | Sí (árbol de secuencias + groups) |
| Scopes (panel vs popup vs insert) | Manual (capas de `Keybinds`) | Nativo (`Scope`) |
| UI discoverability | No (solo lógica) | Popup which-key integrado |
| Crossterm | Feature opcional; **no** reemplaza el event loop | Feature `crossterm`; consume `KeyEvent` / `Event` |
| Licencia | MIT | **LGPL-3.0-or-later** (compatible con GPL-3, pero hay que documentar) |
| Encaje Norton/Far | Excelente para F-keys + config usuario | Excelente si añadimos leader/secuencias y ayuda contextual |

#### Recomendación oficial para Pairee

**Opción A (recomendada): `keybinds` como sistema único de atajos.**

1. Sustituye `src/keybindings/{resolver,preset}` y el string-matching casero de `key_event_to_string`.
2. Crossterm **sigue** capturando eventos de terminal (raw mode, resize, mouse, paste); **deja de “ser” el mapa de atajos**. El flujo queda:

   ```text
   crossterm Event → (KeyPress) → keybinds::Keybinds::dispatch → Action → handlers
   ```

3. Presets Norton/Vim/Modern = tablas `keybinds` cargadas desde TOML (`keybinds` + `serde`).
4. Command palette y F-key bar leen el **mismo** registro de bindings (una sola fuente de verdad).
5. **No** montar un segundo keymap paralelo.

**¿Y `ratatui-which-key`?**

- **No usarlo junto a `keybinds` como dos keymaps completos** (doble verdad, scopes duplicados, peor mantenimiento).
- **Opcional en fase posterior (F.3):** evaluar which-key **solo** si queremos popup de leader/secuencias *después* de migrar a `keybinds`, **o** migrar el input entero a which-key y **no** usar `keybinds`.
- Si en el futuro se prefiere which-key solo: conviene cuando el producto priorice leader-keys y ayuda contextual tipo modal over F-keys clásicas. Hoy Pairee es Norton/Far → F-keys + presets → **`keybinds` primero**.

**Resumen:** implementar **`keybinds` obligatorio**; `ratatui-which-key` **no** en el primer PR (salvo spike de 1 día que demuestre que puede reemplazar todo el stack sin dualismo).

### 6.2 Checklist — keybinds (F.1)

**Problemas que resolvemos (historial Pairee):**

| Problema | Antes | Ahora |
|----------|-------|--------|
| Atajos imposibles (`Ctrl+rj`) | Se aceptaban como strings opacos | **`keybinds` parse → error** (`KeymapLoadReport`) |
| Mismo chord en dos acciones | Last-wins silencioso en `HashMap` | **Conflicto detectado y rechazado** |
| Presets Norton / Vim / VSCode | HashMap monolítico + TOML a medias | **TOML validado** (`keymaps/*.toml` + embed) |
| Crossterm como “mapa” | `key_event_to_string` casero | Crossterm = eventos; **mapa = `keybinds`** |

- [x] Añadir dependencia `keybinds` (`crossterm` + `serde`)
- [x] Loader con validación: parse de chords + detección de duplicados (`loader.rs`)
- [x] Migrar resolver → `Keybinds::dispatch` / `would_trigger`
- [x] Presets Norton / Neovim / VSCode desde TOML (embed + disco)
- [x] Rechazar custom bindings inválidos o en conflicto (log + no insertar)
- [x] Mantener F1–F10 vía `resolve_for_key_string` / inverse map
- [x] Tests: impossible chord, duplicate, norton core keys
- [x] Slim `preset.rs` (solo `parse_action_name` + TOML embed)
- [ ] UI de settings que muestre errores de keymap al usuario (hoy log)
- [ ] Migración documentada de aliases `Gray+` → `Plus` (ya mapeados en loader)

### 6.3 Checklist — `tui-scrollbar` (F.2)

Crate: [`tui-scrollbar`](https://crates.io/crates/tui-scrollbar) (Joshka / tui-widgets). Grok Build ya lo usa en pager-render y textarea.

- [ ] Añadir `tui-scrollbar` al `Cargo.toml` (alineado a `ratatui` 0.30)
- [ ] Integrar en paneles con scroll real:
  - [ ] Help F1 / markdown reader
  - [ ] Viewer F3 / quickview
  - [ ] History lists (commands / folders / files)
  - [ ] Transfer panel log + file list
  - [ ] Plugin menu / config dialog largos
  - [ ] Git log / diff views
- [ ] Mouse drag/jump opcional vía API de interacción del crate (si mouse capture está on)
- [ ] Tema: colores de thumb/track desde `Theme` (sin hardcode)

### 6.4 Estabilidad TUI y anti-glitch (F.3) — lecciones de Grok Build

Problema reportado: UI “funciona pero no termina de quedar bien”; **glitches aleatorios en Windows y Linux**.

#### Causas probables en Pairee hoy

| Síntoma | Causa probable en código actual |
|---------|----------------------------------|
| Parpadeo / frames a medias | `draw` **cada tick ~50 ms** aunque no haya cambios (`app/app/mod.rs`) |
| Destellos al refrescar | `terminal.clear()` completo bajo `terminal_needs_clear` |
| Layout “roto” con Unicode | Cálculo de anchos sin `unicode-width` / segmentación |
| Resize glitchy | Falta de protocolo **synchronized update** (DEC 2026) alrededor del frame |
| Input raro en algunos emuladores | Keyboard enhancement flags + focus change sin degradación unificada |

#### Patrones de Grok Build a **traer** (adaptados, no copiar monorepo)

| Patrón en Grok Build | Aplicación en Pairee | Prioridad |
|----------------------|----------------------|-----------|
| `BeginSynchronizedUpdate` / `EndSynchronizedUpdate` (`crossterm`) alrededor del draw | Envolver `terminal.draw` en sync update → menos tearing | **P0 anti-glitch** |
| Draw **dirty / on-demand** (no pintar si no hay cambio) | Flag `ui_dirty` + redibujar solo tras input/events/background | **P0** |
| `tui-scrollbar` + métricas de thumb fraccional | Ver F.2 | P1 |
| `unicode-width` + `unicode-segmentation` | Truncate/pad de nombres de archivo, columnas de panel | P1 |
| Registro central de acciones (`ActionDef`: id, label, keys, category, when) | Unificar F-keys + palette + help de atajos | P1 (junto a keybinds) |
| `bracketed-paste` en crossterm | CLI / rename / apply-command sin basura de paste | P1 |
| `ansi-to-tui` | Panel de terminal / salida de apply-command con ANSI | P2 |
| `shlex` | Parse seguro de comandos usuario (apply / user menu) | P2 |
| Terminal capability detection (brand, KKP unreliable) | Degradar features en conhost / tmux viejo | P2 |
| Tests PTY e2e de render | Smoke resize + draw en CI | P2 |
| `xai-ratatui-inline` (viewport inline + scrollback nativo) | **No** copiar de entrada: es chat/REPL, no dual-panel fullscreen | Fuera de alcance |
| Mermaid / markdown heavy stack | Solo si un día el help necesita más; hoy `pulldown-cmark` basta | Fuera / P3 |

#### Checklist anti-glitch

- [x] Envolver frame draw en synchronized update (`Begin/EndSynchronizedUpdate`)
- [x] Introducir `ui_dirty` / skip draw cuando no hace falta pintar
- [x] Rate-limit redraw de progreso de transfer (~12 Hz)
- [ ] Evitar `clear()` full-screen salvo resize o cambio de screen mode (reducir usos restantes)
- [ ] `unicode-width` en listados de paneles y popups
- [ ] Revisar `KeyboardEnhancementFlags` / focus: feature-detect
- [ ] Checklist manual Windows Terminal + conhost + Linux

### 6.5 Otras librerías de Grok Build candidatas (evaluación)

| Crate / idea | ¿Traer a Pairee? | Notas |
|--------------|------------------|-------|
| `tui-scrollbar` | **Sí** | F.2 |
| `unicode-width` / `unicode-segmentation` | **Sí** | layout estable |
| `keybinds` | **Sí** | F.1 |
| `ratatui-which-key` | Spike / opcional | Solo si no dual-keymap (ver 6.1) |
| `ansi-to-tui` | Candidato | terminal embebido / logs con color |
| `nucleo` (fuzzy) | Candidato | mejorar command palette / find file |
| `tracing` (+ subscriber) | Candidato | sustituir o complementar `simplelog` |
| `arboard` | Candidato | copy path al clipboard del SO |
| `signal-hook` | Candidato Unix | SIGWINCH / graceful signals |
| `xai-ratatui-textarea` | No (interno monorepo) | Evaluar `tui-textarea` público si hace falta editor embebido |
| `alacritty_terminal` | No de momento | PTY embebido completo es otro producto |

### 6.6 Orden de implementación sugerido (F)

1. **F.3a** Synchronized update + dirty draw (ROI glitches, bajo riesgo de producto)
2. **F.1** Migración `keybinds` + presets TOML + borrar resolver casero
3. **F.2** `tui-scrollbar` en viewers/help/history/transfer
4. **F.3b** unicode-width + paste + rate-limit progress redraw
5. Spike which-key **solo** si tras F.1 se echa de menos discoverability de secuencias

---

## 7. Fase C — Estado y modularidad (P1)

**Patrones:** State objects, Dialog stack, Facade de servicios, Event bus.  
**Nota:** tras Fase F, `keybindings/preset.rs` se reduce o desaparece; priorizar monólitos UI restantes.

- [ ] Extraer de `AppState`: `TransferUiState`, `UpdateState`, `PluginHostState`, `HistoryState`, `PanelPair`
- [ ] Reemplazar mega-enum `PopupType` por stack de diálogos tipados (un módulo por familia)
  - Nota: hoy tiene `#[allow(clippy::large_enum_variant)]` temporal
- [ ] Partir archivos >500 LOC (lista baseline abajo)
- [ ] Sustituir poll `take/unwrap/put-back` de receivers por bus o `select!` idiomático
- [ ] Valorar `src/lib.rs` + binario fino para tests/benches

### Archivos monolíticos a partir (baseline)

| Archivo | ~LOC | Estado |
|---------|------|--------|
| `src/fs/transfer/worker.rs` | ~1037 | [x] → `worker/` multi-módulo |
| `src/app/input_popup/plugin_menu/dev/options.rs` | ~793 | [ ] |
| `src/app/actions/ui_settings.rs` | ~766 | [ ] |
| `src/ui/popup/history_lists.rs` | ~742 | [ ] |
| `src/ui/transfer/panel.rs` | ~676 | [ ] |
| `src/keybindings/preset.rs` | ~602 | [ ] → **absorber en Fase F.1 (`keybinds`)** |
| `src/app/state/types.rs` | ~598 | [ ] |
| `src/plugin/updater.rs` | ~552 | [ ] |
| `src/fs/list.rs` | ~521 | [ ] |
| `src/config/settings.rs` | ~518 | [ ] |

---

## 8. Fase D — Plugins productivos (P2)

Basado en `docs/technical/plugin-roadmap.md` (G1–G14).

- [ ] Diálogos reales end-to-end (confirm/input/select)
- [ ] Userdata tipados (`File`, metadata, mime)
- [ ] API async FS + `Command` builder con streaming
- [ ] Contexto vivo `cx` (no solo snapshot)
- [ ] Plugins de aceptación en CI
- [ ] API docs versionadas (semver de superficie Lua)
- [ ] Actualizar README + help de plugins

---

## 9. Fase E — Producto y distribución (P3)

- [x] Command palette sobre `Action` (`Ctrl+Shift+P`, filtro + Enter)
- [ ] Onboarding / primer arranque (elegir preset de teclas) — **después de F.1 keybinds**
- [ ] Más idiomas (pipeline localize-helper)
- [ ] Feature flags (`ssh`, `git`, `plugins`, `image-preview`)
- [ ] CI macOS si se declara soporte oficial
- [ ] Threat model corto (plugins, SSH, update, elevated helper)
- [ ] Fuzzing parsers (config TOML, manifests, descript.ion, globs)

---

## 10. Métricas objetivo (3 meses)

| Métrica | Baseline | Objetivo | Actual |
|---------|----------|----------|--------|
| CI en rama default | No | Sí | **Sí (`master`/`main`)** |
| Platforms en CI | Linux (mal cableado) | Linux + Windows | **Sí** |
| Clippy crate allow all | Sí | No | **No** |
| Tests | 115 unit | 115+ y ≥15 integration | **116 unit + 4 integration** |
| Archivos >800 LOC | ≥2 | 0 | worker.rs eliminado; quedan monólitos UI |
| Docs con status real | Desfasadas | Índice OK | **Índice + banners** |
| Transfer dual path | Sí | Engine unificado | **Hecho (Fase B)** |
| Command palette | No | Sí | **Sí** |
| Keymap stack | Casero crossterm strings | `keybinds` validado | **Hecho F.1** |
| Scrollbars | Ratatui default / ninguno | `tui-scrollbar` en listas largas | **Pendiente F.2** |
| Glitches TUI Win/Linux | Presentes | Sync update + dirty draw | **Parcial F.3a** |

---

## 11. Registro de commits de este plan

| Fecha | Commit | Qué se hizo |
|-------|--------|-------------|
| 2026-08-12 | `a8bc062` … `9b9c5a0` | Fases A–B: higiene, CI, clippy, transfer engine, purge legacy |
| 2026-08-12 | `007eca7` | docs: plan Fase F |
| 2026-08-12 | _(este)_ | feat: `keybinds` + validación + dirty/sync draw |

Ver también `git log --oneline master` para el detalle.

---

## 12. Roadmap visual

```text
Alto impacto │  [x CI] [x Clippy] [x Transfer unificado]
             │  [→ keybinds + anti-glitch] [→ tui-scrollbar]
             │  [ Partir AppState/PopupType ] [ Plugins ]
             │  [x Docs sync] [ Feature flags ]
Bajo impacto │  [ Más idiomas ] [ which-key opcional ] [ macOS CI ]
             └────────────────────────────────────────────
               Bajo esfuerzo              Alto esfuerzo
```

---

## 13. Conclusión operativa

**Fase A y B cerradas.**  
**F.1 keybinds + F.3a anti-glitch (parcial) implementados.**  
Siguiente: **F.2 tui-scrollbar**, unicode-width, y pulido anti-glitch.  
`ratatui-which-key` sigue opcional (no dual-keymap).

---

*Última actualización del progreso: 2026-08-12 (F.1 keybinds + F.3a sync/dirty draw).*
