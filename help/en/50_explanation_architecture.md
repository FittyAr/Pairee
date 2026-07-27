# 💡 Explanation: Architecture, Async Transfers, and Screens

> **Quadrant: EXPLANATION** — *understanding-oriented. Discusses a topic; you do not need to read this to use Pairee.*

This page is for the curious. It describes the moving parts under the
hood — the **asynchronous runtime**, the **transfer engine**, and the
**screen stack** — and explains *why* Pairee never freezes even when
you copy a 50 GB folder.

---

## 1. The runtime: `tokio` + `crossterm`

Pairee is built on:

- **`ratatui`** — the TUI rendering layer.
- **`crossterm`** — the cross-platform terminal backend (input parsing,
  alternate screen, raw mode).
- **`tokio`** — the asynchronous runtime. Every long-running operation
  (copy, move, delete, wipe, search, archive, git, ssh, update check,
  plugin load) is spawned as a `tokio` task.

The main thread **never blocks on I/O**. It owns the terminal and the
UI state. Background workers send **progress events** over `mpsc`
channels; the main thread drains the channel between draws.

The benefit: while a 50 GB copy is in progress, you can navigate
panels, edit a file, switch screens, open the help popup, or fire
another transfer. The UI remains at 60 fps.

---

## 2. The transfer engine

Located under `src/fs/transfer/`, the transfer engine is a small
pipeline:

```
        ┌─────────────┐
src ──▶ │   filter    │──▶  pre-conditions: glob, overwrite policy
        └─────────────┘
                │
                ▼
        ┌─────────────┐
        │   pipeline  │──▶  bytes flow through buffered I/O
        └─────────────┘
                │
                ▼
        ┌─────────────┐
        │  metadata   │──▶  ownership, mtime, xattrs
        └─────────────┘
                │
                ▼
        ┌─────────────┐
        │ post_action │──▶  hook for plugins / shell-out
        └─────────────┘
                │
                ▼
              dst
```

### Components

- **`filter.rs`** — applies user globs and overwrite policies.
- **`pipeline.rs`** — streams bytes in chunks (default 256 KiB)
  through an async reader/writer pair. The default is the pure-Rust
  pipeline. Setting `transfer_engine = "direct"` in `settings.toml`
  switches to the OS copy API (faster on some platforms, but
  overwrites without policy enforcement).
- **`metadata.rs`** — preserves Unix permissions, timestamps, and
  extended attributes when `preserve = true` is set.
- **`hash/`** — verifies integrity. SHA-256, SHA-1, BLAKE3, MD5,
  CRC32 are all available. Used for the update system's signature
  check, the secure-wipe verification, and the file-cache key.
- **`engine.rs`** — the orchestrator. Holds the job queue, manages
  worker concurrency, and emits events to the UI.
- **`events.rs`** — the message types. `Started`, `Progress`,
  `Completed`, `Failed`, `Cancelled`.

### Conflict resolution

When a destination already exists, the engine consults the **conflict
policy** in this order:

1. The per-operation `overwrite` argument (Copy / Move dialog).
2. The `Configuration → Confirmations → Confirm overwrite` setting.
3. The default — **prompt** the user.

The prompt dialog offers:

| Button | Result |
| --- | --- |
| **Overwrite** | Replace the destination file. |
| **Overwrite all** | Replace all subsequent conflicts. |
| **Skip** | Keep the destination, continue with the next file. |
| **Skip all** | Skip all subsequent conflicts. |
| **Append** | Concatenate (where it makes sense). |
| **Cancel** | Abort the entire job. |

### Cancellation

The engine watches a `CancellationToken` per job. UI buttons
("Cancel" in the progress popup) flip the token; workers poll it on
each chunk and abort cleanly. Already-copied bytes are **not**
rolled back, but the engine writes a `cancelled` record to the
transfer history so you know what happened.

### Concurrency

Multiple jobs can run in parallel. The number of concurrent workers
defaults to `min(num_cpus, 4)` and is tunable per-operation. The
**Transfer Panel** (`Ctrl+T`) shows the live state of every job.

---

## 3. The screen stack

A "screen" in Pairee is one of the things the UI can render at the
top: a **panel screen**, the **editor**, the **viewer**, the
**Git dashboard**, the **screens overlay**, a **dialog**, etc.

Screens are organised as a **stack**:

```
┌───────────────────────────────┐
│ Screen N (top)                │  ← active
├───────────────────────────────┤
│ ...                           │
├───────────────────────────────┤
│ Screen 0 (bottom)             │  ← the first panel view
└───────────────────────────────┘
```

When you press `F4` on a file, a new editor screen is **pushed**.
When you press `F10` to quit the editor, it is **popped** and the
panel screen below becomes active again. State (cursor, selection,
buffer) is preserved.

### Suspending popups

A subtle feature: when a popup is open (say, the copy destination
dialog) and you press `F12` to open the Screens overlay, the dialog
is **suspended** (its state is saved) while you interact with the
overlay. When you return, the dialog is exactly where you left it.

This makes it possible to start a copy, jump to the editor to fix a
typo, jump to the viewer to confirm a file, then come back and
confirm the copy — all without losing the prompt.

### The F-key bar

The bottom F-key bar reflects the **top screen**:

- In a **panel** screen: F1 Help, F2 User, F3 View, F4 Edit, …
- In the **editor** screen: F1 Help, F2 Save, F4 Hex, F7 Search,
  F8 Discard, F10 Quit.
- In the **viewer** screen: F1 Help, F4 Hex, F7 Search, F10 Quit.

The bar is purely visual; the actual keybindings work regardless of
the displayed row. (SSH terminals that do not report modifier
state can use `Ctrl+P` to lock the bar in the row they want — see
[`30_howto_ssh_modifier_keys`](30_howto_ssh_modifier_keys.md).)

---

## 4. Why this matters to you (the user)

The architecture has three direct consequences you will feel:

1. **The UI never freezes.** If something seems stuck, the most
   likely cause is the terminal (try `Ctrl+L` to force a redraw) or
   a network mount (try `Ctrl+R` to refresh only the active panel).

2. **You can queue many operations.** Press `F5` on a folder, then
   `F5` on another, then `F5` on a third. They all run in parallel,
   and the Transfer Panel tracks each one independently.

3. **Cancellation is cheap and safe.** You can abort a long job at
   any time. The engine stops at the next chunk boundary, writes a
   cancel record, and you can clean up partial files with a normal
   delete.

---

## 5. Where to look in the code

If you are a developer curious about the code:

- `src/fs/transfer/mod.rs` — the engine entry point.
- `src/fs/transfer/pipeline.rs` — the streaming pipeline.
- `src/app/state/screens.rs` — the screen stack.
- `src/app/app/events.rs` — how background events reach the UI.
- `src/plugin/runtime/runtime.rs` — the Lua plugin runtime.
