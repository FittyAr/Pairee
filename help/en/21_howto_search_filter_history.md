# 🔧 How-To: Search, Filter, History, and Hotlist

> **Quadrant: HOW-TO** — *problem-oriented.*

Four everyday needs share this page: finding a file by name, finding a
file by content, narrowing what the current panel shows, and jumping
to a place you have been before.

---

## 1. Find files by name (Alt+F7)

**Goal:** search one or more directories for a file whose name matches
a pattern.

1. Press `Alt+F7` (or `Commands → Find file`).
2. In the dialog, set:
   - **Search pattern** — a glob like `*.toml`, a substring like
     `cargo`, or a brace pattern like `*.{rs,toml}`.
   - **Search in** — the root directory (defaults to the active panel).
   - **Include subdirectories** — toggle recursion.
   - **Case sensitive** — toggle.
   - **Search content** — if checked, also scan file bodies for a
     literal string (see Section 2).
3. Press `Enter`. Results stream into a results list.
4. In the results list:
   - `Up` / `Down` to move.
   - `Enter` to **jump** to the file in the active panel.
   - `Esc` to close the results without changing the panel.

---

## 2. Find files by content (Alt+F7 with content)

**Goal:** find every file that contains a given text.

1. Press `Alt+F7`.
2. Check **Search content**.
3. Type the literal text in the **content** field. Glob patterns here
   are matched by the `Search pattern` field; the content field is a
   plain literal substring.
4. Press `Enter`.
5. Results show each file that contains the string, one entry per match
   (per line in some cases). Press `Enter` on a row to jump to it.

> The content search is **literal** (no regex). It is fast enough for
> tens of thousands of lines on a local SSD.

---

## 3. Filter the active panel (Ctrl+I)

**Goal:** restrict the panel to a glob, persistently.

1. Focus the panel you want to filter.
2. Press `Ctrl+I` (or `Commands → File panel filter`).
3. Type a glob (e.g. `*.rs`).
4. Press `Enter`. The panel now shows only matching items.

To clear the filter, press `Ctrl+I` again and submit an empty string
(or `*`).

> The filter survives panel refresh and panel swaps, but resets when
> the panel path is changed manually.

---

## 4. Quick filter (Ctrl+F or f / F)

**Goal:** filter the panel *while you type*, with no commit step.

1. Press `Ctrl+F` (or `f` / `F` from the panel).
2. The bottom strip becomes an input. Every character you type
   narrows the panel to files whose names contain the substring.
3. Press `Esc` to release the filter and restore the full listing.

> Quick filter is **substring** by default. The full dialog (Ctrl+I)
> is glob-based.

---

## 5. Jump to a directory hotlist entry (Ctrl+\)

**Goal:** bookmark a folder, then jump back to it later.

1. Navigate to a directory.
2. Press `Ctrl+\` (or `Commands → Folder shortcuts`).
3. The dialog lists your saved shortcuts. Use:
   - `Insert` to **add** the current panel path to the list.
   - `Enter` to **jump** to the highlighted entry.
   - `Delete` to **remove** the highlighted entry.
   - `e` to **rename** the highlighted entry.

Shortcuts are stored in your config folder as
`hotlist.toml` (or similar). They persist across sessions.

---

## 6. Recent folders (Alt+F12)

**Goal:** reopen a directory you have visited recently.

1. Press `Alt+F12` (or `Commands → Folders history`).
2. The dialog lists recent paths, newest first.
3. `Enter` to jump to the highlighted path; `Delete` to drop it from
   the history.

> The history is recorded only if **Save folders history** is enabled
> in `Configuration → System`.

---

## 7. Recent files in viewer and editor (Alt+F11)

**Goal:** reopen a file you recently viewed or edited.

1. Press `Alt+F11` (or `Commands → File view history`).
2. The dialog lists files you have opened with `F3` or `F4`, newest
   first.
3. `Enter` to view it again.

> Controlled by the **Save view and edit history** option in
> `Configuration → System`.

---

## 8. Command-line history (Alt+F8)

**Goal:** recall a previous typed command (e.g. a path you typed on the
command line).

1. Press `Alt+F8` (or `Commands → History`).
2. A history popup lists your last commands. `Enter` to re-execute,
   `Esc` to close.

> Controlled by **Save commands history** in
> `Configuration → System`.

---

## 9. Tree view (Alt+F10)

**Goal:** browse the directory structure of the current path as a
graph.

1. Press `Alt+F10` (or `Commands → Tree view`).
2. The tree overlay shows the directory tree of the active panel's
   path. Use arrows to navigate, `Enter` to dive in, `Esc` to close.

> Useful when you want a visual overview before drilling into a deep
> folder.

---

## Where to go next

- Reference for the F-key bar: [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
- Settings that gate history: [`41_reference_configuration`](41_reference_configuration.md)
