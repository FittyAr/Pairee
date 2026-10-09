# Keyboard Shortcuts

Pairee does not ask you to learn a new keyboard layout. Pick the preset of
the program your fingers already know, and every key works the way it did
there. This page explains how the keys work; the full list of each preset is
in the **Keys:** pages of this help, and inside Pairee in the keyboard
shortcuts list.

---

## 1. Presets

| Preset | For people coming from | In short |
| :--- | :--- | :--- |
| **Norton Commander / Far** | Norton Commander, Far Manager, Midnight Commander | `F3` view, `F5` copy, `F6` move, `F7` folder, `F8` delete, typing goes to the command line, `Alt`+letter quick search |
| **Standard** | Windows Explorer, VS Code, Total Commander | `Ctrl+C/X/V` files, `Delete` to the recycle bin, `F2` rename, `Ctrl+T` tabs, typing jumps to a file |
| **Neovim** | Vim, Neovim, vifm, oil.nvim, nvim-tree | `hjkl`, `gg`/`G`, `yy`/`dd`/`p`, `/` `n` `N`, `Space` leader |
| **yazi** | yazi | `hjkl`, `y`/`x`/`p`, `d`/`D`, `a` create, `.` hidden files, `~` keys |

Choose one the first time Pairee starts, or later in **Options → Configuration
→ Interface → Keybindings preset**, or in the keyboard shortcuts list
(`Ctrl+←`/`Ctrl+→`, then `F5`). The Neovim and yazi presets keep every Norton
Commander F-key, so `F5` still copies.

The page **Coming from another program** lists what is the same and what
differs for each of them.

---

## 2. The keyboard shortcuts list

Open it with the key your preset gives it (`Ctrl+K` in Norton, `Ctrl+K Ctrl+S`
in Standard, `g?` in Neovim, `~` in yazi; `Ctrl+Shift+K` in all of them on
terminals that report it), or from **Options → Configuration → Interface →
Keyboard shortcuts…**.

It shows every command of the panels, the editor, the viewer and the lists in
dialogs (`Tab` switches between them), grouped by category, with the commands
that have no key and those of your plugins. Marks before a row:

| Mark | Meaning |
| :--- | :--- |
| `●` | You changed this command's keys. |
| `⚠` | A plugin suggested a key that is already used, so it has none. |
| `≈` | Its keys need a capable terminal (see section 5). |
| `∅` | No key: use the command palette or give it one. |

| Key | In the list |
| :--- | :--- |
| *typing* | Filters by name, id or key. |
| `F3` | Press any key to see what it does. |
| `Enter` | Runs the highlighted action. |
| `F2` | Gives the action a new key: press it (or a sequence) and `Enter`. If another action uses it, Pairee asks first. |
| `Ins` | Adds one more key to the action. |
| `Del` | Removes the action's keys. |
| `F8` / `Shift+F8` | Gives the action (or every action) back the preset's keys. |
| `Ctrl+←` / `Ctrl+→` | Shows another preset; `F5` makes it the active one. |
| `F9` | Saves the preset with your changes as a preset file of your own. |

Changes are saved at once and work immediately. The **command palette**
(`Ctrl+Shift+P`, `Ctrl+Alt+P`, `:` in Neovim) lists every action with its key.

---

## 3. How keys work

- **Chords** are a key with modifiers: `F5`, `Ctrl+c`, `Alt+Shift+PageUp`. An
  uppercase letter means `Shift` (`Ctrl+K` is `Ctrl+Shift+k`).
- **Sequences** are several keys pressed one after the other: `g g`,
  `Ctrl+w h`. While one is under way, the keys that can follow are shown at
  the bottom of the screen; `Esc` cancels it. A sequence waits one second for
  its next key (yazi waits as long as needed).
- **The leader key** (Neovim: `Space`) starts most Neovim sequences:
  `Space f f` finds files.
- **Letters** do what your old program did with them: in Norton they type
  into the command line, in Standard they jump to the file whose name starts
  with them, and in Neovim and yazi they are commands (a letter without a
  command does nothing; `!`, `;` or `:` open the command line).
- **Quick search** (Norton): `Alt` + a letter searches by name; keep typing,
  `↑`/`↓` move between matches, `Esc` closes.
- **Contexts**: the panels, the editor, the viewer and the lists in dialogs
  have keys of their own. Help, the screens list, the command palette and
  the shortcuts list work everywhere.

---

## 4. Changing keys in files

Everything the shortcuts list does is written to `keybindings.toml` in the
configuration folder, which you can also edit:

```toml
preset = "norton"

[overrides.all]            # every preset
copy = "F5, Ctrl+Alt+c"    # several keys: separated by commas
quick_view = ""            # no key

[overrides.norton]         # only this preset
"editor.save" = "Ctrl+s"   # editor, viewer and list keys have a prefix
"viewer.quit" = "q, Esc"
"plugin.git-blame.toggle" = "Ctrl+Alt+g"
```

Naming an action replaces its keys, and a key another action had moves to
it. A preset of your own goes in the `keymaps` folder (`F9` in the list
writes one) and only lists what it changes:

```toml
extends = "neovim"

[options]
typing = "commands"        # cli | type_ahead | commands
leader = "Space"
sequence_timeout = 1000    # milliseconds; 0 waits as long as needed
alt_quick_search = false

[panels]
go_parent = "h, -"

[editor]
save = "Ctrl+s, F2"
```

Problems in these files (an unknown action, a key that cannot be typed, two
actions on one key) are listed in **Options → Configuration → Interface →
View keymap issues…**.

---

## 5. Terminal limits

Some keys only arrive when the terminal reports them, and the list marks
such keys with `≈`:

- `Ctrl+Shift+letter`, `Ctrl+Enter`, `Ctrl+Tab` and `Ctrl+digit` need a
  terminal with the kitty keyboard protocol (kitty, WezTerm, foot, Ghostty,
  Windows Terminal).
- `Ctrl+i`, `Ctrl+m`, `Ctrl+h` and `Ctrl+[` are the same as `Tab`, `Enter`,
  `Backspace` and `Esc` on older terminals.
- On Windows `Ctrl+Alt` is `AltGr`: with some keyboard layouts
  `Ctrl+Alt+2` or `Ctrl+Alt+E` type a character instead.
- Some terminals keep `F11`, `Alt+F4` or `Alt`+arrows for themselves.

Every essential action has at least one key that works everywhere, and every
action is also in the menu bar (`F9`/`F10`) and in the command palette.

---

## 6. Plugin keys

Plugins suggest keys for their commands. A key your preset already uses is
never taken: the command stays without a key (marked `⚠` in the list) and
you can give it one. See **Plugins** for writing plugins with keys.

---

## 7. Editor and viewer

Pairee always edits files with its built-in editor; no external editor is
launched. Its commands (save, search, copy, undo, close...) and every viewer
key come from your preset and are listed in the **Keys:** pages and in the
shortcuts list. Moving and selecting text works the same in every preset:

| Key | In the editor |
| :--- | :--- |
| Arrows, `Home` / `End`, `PgUp` / `PgDn` | Move; `Ctrl+Home` / `Ctrl+End` go to the start / end of the file. |
| `Shift` + those keys | Select text. Dragging with the mouse also selects. |
| `Alt+Shift` + arrows / `Home` / `End` (also `Ctrl+Alt+Shift`) | Vertical block (column) selection, Far style. `Alt` + mouse drag also selects a block. |
| Typing, `Backspace`, `Delete` | Replace / delete the selection. |
| `Tab` | Insert a tab, or spaces up to the next tab stop when **Expand tabs** is on. |

Block mode (`Ctrl+B` in every preset) makes `Shift` + arrows and mouse drags
select a vertical block, for terminals that keep `Alt+Shift` + arrows for
themselves, such as Windows Terminal.

The editor keeps the file's line endings (LF or CRLF), final newline and
UTF-8 BOM. Files that are not valid UTF-8 or larger than 64 MiB are not
opened (use the viewer instead). Copy and cut use the system clipboard; when
it is not available (over SSH or on a headless Linux session) Pairee keeps
the text in its own clipboard, so copy and paste still work between editor
screens.

The viewer reads files page by page, so even multi-gigabyte files open at
once. Its status line shows the encoding, the current line (or hex offset)
and the progress of indexing or of a search. The encoding is detected
automatically (byte-order mark, UTF-16, UTF-8, else the most likely legacy
code page); the viewer's encoding command picks another one, and detection
can be turned off in **Options → Configuration → Editor/Viewer**.

---

## 8. Keys inside dialogs

Lists in dialogs (menus, history, pickers) move with the arrows, `PgUp` /
`PgDn` and `Home` / `End`, `Enter` chooses and `Esc` closes; the Neovim and
yazi presets add `j` / `k`, `gg` / `G` and `q`. Some dialogs have keys of
their own, shown at their bottom line. The file associations editor:

| Key | Action |
| :--- | :--- |
| `A` / `a` / `Insert` | Add a rule. |
| `E` / `e` / `Enter` | Edit the highlighted rule. |
| `D` / `d` / `Delete` | Delete the highlighted rule. |
