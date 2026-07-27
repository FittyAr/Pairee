# 🎓 Tutorial: Panels, View Modes, and Screens

> **Quadrant: TUTORIAL** — *learning-oriented, hands-on.*

This tutorial teaches the three spatial concepts you will use every
day: the **two panels**, the **nine view modes**, and the **multi-screen
background**. Master these and you master 70% of Pairee.

---

## 1. The two panels

Pairee's defining feature is the **dual-panel** layout inherited from
Norton Commander. Both panels show a directory listing at all times;
one is *active* (highlighted border, your keystrokes apply here), the
other is *passive* (it shows where your next copy/move will land).

### Which panel is active?

- The active panel has a **thicker / differently-coloured border**.
- The clock and menu bar sit on top; the F-key bar sits at the bottom.
- The active panel's path is shown at the top of its listing.

### Switching focus

| Key | Effect |
| --- | --- |
| `Tab` | Toggle focus between the two panels. |
| `Shift+Tab` | Same, in the opposite direction. |
| `Ctrl+U` | **Swap** the two panels (paths exchange sides). |
| `Ctrl+O` | **Hide both** panels. Press again to bring them back. Useful to inspect background output. |
| `Ctrl+F1` | Toggle visibility of the **left** panel only. |
| `Ctrl+F2` | Toggle visibility of the **right** panel only. |

### Why two panels?

Because almost every file operation is a **source → destination** task,
and the two panels make source and destination *visible at the same
time*. This is why `F5` (copy) and `F6` (move) act on the active panel
**into** the passive panel — no separate "destination picker" required.

---

## 2. The nine view modes

Each panel can render the same directory in nine different ways. Pick
the one that matches the task at hand.

| `Ctrl+N` | Mode | What it shows |
| --- | --- | --- |
| `Ctrl+1` | **Brief** | Names only, multiple columns. Best for thousands of files. |
| `Ctrl+2` | **Medium** | Name and extension, side by side. |
| `Ctrl+3` | **Full** | Name, size, date modified. The everyday default. |
| `Ctrl+4` | **Wide** | Wide single-column list, more characters per name. |
| `Ctrl+5` | **Detailed** | Unix permissions, owner, group, hardlinks, real size. |
| `Ctrl+6` | **Descriptions** | Name + the description from `Descript.ion` (if any). |
| `Ctrl+7` | **File Owners** | Name + user/group columns. |
| `Ctrl+8` | **File Links** | Name + hardlink count. |
| `Ctrl+9` | **Alt Full** | User-defined column layout. |

> All nine modes honour the same panel filter and tagging; only the
> visual layout changes.

### Sorting

Sort columns are reachable via the **`Ctrl+F3 … Ctrl+F12`** row:

| Key | Sort |
| --- | --- |
| `Ctrl+F3` | Name |
| `Ctrl+F4` | Extension |
| `Ctrl+F5` | Write time (mtime) |
| `Ctrl+F6` | Size |
| `Ctrl+F7` | Unsorted (filesystem order) |
| `Ctrl+F8` | Creation time (birthtime) |
| `Ctrl+F9` | Access time (atime) |
| `Ctrl+F10` | Description |
| `Ctrl+F11` | Owner |
| `Ctrl+F12` | Open the Sort Modes dialog (compound sorts, multiple columns) |

Toggle **reverse sort** with `Ctrl+Shift+R` (and adjust other sort
options in `Configuration → Panel`).

### Filtering the panel

If you only want to see certain files, you have two complementary tools:

| Action | When to use it |
| --- | --- |
| **File Panel Filter** (`Ctrl+I`) | Persistent: stays active until you clear it. Use for sustained focus on a subset. |
| **Quick Filter** (`Ctrl+F` or `f` / `F`) | Live, in-place: the moment you type, the panel filters; press `Esc` to release. |

Both accept globs (`*.rs`, `*.{toml,yaml}`) and substrings. See
[`21_howto_search_filter_history`](21_howto_search_filter_history.md)
for the full set of options.

---

## 3. The multi-screen system

A **screen** is one of the things Pairee can be "doing" right now: a
panel view, an editor, a viewer, a file-attributes popup, etc. You can
have **many open at the same time**, and switch between them without
losing state.

### Open screens

- Press `F4` to open a text file in the **internal editor** — a new
  screen is created.
- Press `F3` to open a file in the **internal viewer** — a new screen.
- Press `F12` to open the **Screens overlay**, which lists every open
  screen. The active one is marked with `*`.

### Switching screens

| Key | Effect |
| --- | --- |
| `F12` | Open the Screens overlay; arrows + `Enter` to jump. |
| `Ctrl+Tab` | Cycle to the **next** screen. |
| `Ctrl+Shift+Tab` | Cycle to the **previous** screen. |
| `Esc` | Close the current popup / unfocus the command line. |
| `F10` (or `Ctrl+Q` in some presets) | **Quit** Pairee (closes everything). |

### State preservation

If you start a copy operation (`F5`), then jump to the editor to fix
a typo, then come back — the copy is still running and its progress
popup is exactly where you left it. Screens **suspend and resume**
popups, search inputs, and command lines, so no work is lost.

### Screens and the F-key bar

The bottom F-key bar reflects what is on the **active screen**:

- In a **panel screen**: `1 Help  2 User  3 View  …`
- In the **editor** screen: `1 Help  2 Save  4 Hex  7 Search  8 Discard  10 Quit`
- In the **viewer** screen: `1 Help  4 Hex  7 Search  10 Quit`

You can always close the active screen with `F10`/`Ctrl+Q` (it will
ask for confirmation if the editor has unsaved changes).

---

## 4. Side panels: Quick View and Info

Two side panels overlay the *passive* panel for the duration they are
open:

| Side panel | Hotkey | Use case |
| --- | --- | --- |
| **Quick View** | `Ctrl+Q` | Instantly preview the highlighted file (text or archive listing) without opening a real screen. |
| **Info Panel** | `Ctrl+L` | Show a small status overlay with hostname, OS, RAM, environment. |
| **Transfer Panel** | `Ctrl+T` | List of currently running and finished background transfer jobs. |

Press the same key again to close.

---

## 5. Quick reference card

```
┌───────────────────────┬───────────────────────┐
│  Left panel (active)  │  Right panel (passive)│
│  /home/me/projects    │  /home/me/backup      │
├───────────────────────┼───────────────────────┤
│ 📁 docs/              │ 📁 docs/              │
│ 📁 src/               │ 📁 notes/             │
│ 📄 README.md          │ 📁 .cache/            │
│ 📄 Cargo.toml         │ 📄 old.zip            │
│ ...                   │ ...                   │
├───────────────────────┴───────────────────────┤
│ 1 Help  2 User  3 View  4 Edit  5 Copy  ...   │
└───────────────────────────────────────────────┘
```

- Press `Tab` to move the active border.
- Press `Ctrl+U` to swap the two paths.
- Press `F12` to manage many screens at once.

---

## 6. Where to go next

- Learn to **find** things: [`21_howto_search_filter_history`](21_howto_search_filter_history.md)
- Learn to **edit, copy, delete** files: [`20_howto_file_operations`](20_howto_file_operations.md)
- See the full key binding list: [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
