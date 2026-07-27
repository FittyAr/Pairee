# 🔧 How-To: Appearance, Themes, and the F-Key Bar

> **Quadrant: HOW-TO** — *problem-oriented.*

Pairee's look is controlled by three things: the **theme**, the
**layout / panel options**, and the **F-key bar** at the bottom. This
page covers how to change them.

---

## Switch theme

### From the Configuration dialog

1. Press `F9` → `Options` → `Configuration` (or `Commands →
   Configuration`).
2. Move to the **Colors** tab.
3. The first row toggles the bundled theme between **`slate`** and
   **`classic_blue`**.
4. `Enter` to apply; `F9` again to save.

### With a custom theme file

Custom themes are loaded from:

- **Windows:** `%APPDATA%\pairee\config\themes\`
- **Linux / macOS:** `~/.config/pairee/themes/`

Drop a `.toml` file in that folder, then select it from the **Colors**
tab. The full schema is documented in
[`42_reference_themes`](42_reference_themes.md).

---

## Edit color groups

Color groups are named slots you can target in your custom theme (for
example, the colour of `directory` listings, of `executable` listings,
of the `border` of the active panel, etc.).

1. Press `F9` → `Options` → `Configuration`.
2. **Colors** tab → second row → **Color groups**.
3. A modal lists every group. Pick one and `Enter`.
4. Choose a colour by name (`Blue`, `Yellow`, …) or hex (`#RRGGBB`).
5. `Enter` to apply; `Esc` to cancel.

---

## Files highlighting

The third row in the **Colors** tab opens the **Files highlighting**
editor. There you can map file masks to colours:

| Mode | Meaning |
| --- | --- |
| `+H` | Hidden / system files |
| `+S` | Symlinks |
| `+D` | Directories |
| `<exec>` | Executables |
| `<arc>` | Archives |
| `<temp>` | Temp files |

You can add custom rules with a glob (`*.rs`) and a colour.

---

## Layout and panel options

Press `F9` → `Options` → `Configuration` → **Panel** tab. The options
are split into:

- **Display & selection**: show hidden files, highlight files, select
  folders, right-click selects.
- **Sorting**: sort by extension, reverse, show sort letter.
- **Updates & information**: object count throttling, network drive
  auto-refresh, free size, total files info.
- **Appearance**: column titles, status line, scrollbar, background
  screens count, ".." in root.
- **Info panel & descriptions**: hostname format, Descript.ion
  options.

For the full field reference see
[`41_reference_configuration`](41_reference_configuration.md), Tab 1.

---

## Interface options

Press `F9` → `Options` → `Configuration` → **Interface** tab:

- **Clock** — show a live digital clock in the top-right.
- **Mouse support** — toggle mouse navigation and clicks.
- **Show bottom F-keys bar** — toggle the F1–F12 hint bar.
- **Always show the menu bar** — keep the top menu visible.
- **Screen saver minutes** — auto-blank after idle.
- **Total copy / delete progress** — show aggregated progress and ETA.
- **Use Ctrl+PgUp to change drive** — alternate drive-switch keybind.
- **Use virtual terminal** — Windows console mode toggle.
- **ClearType friendly redraw** — works around font glitches on
  Windows Console.
- **Window Title Format** — tokens for the terminal title bar.
- **Enable Yazi workflow** — `s` opens Sort, `v` opens View, only when
  the command line is empty.

For the full field reference see
[`41_reference_configuration`](41_reference_configuration.md), Tab 2.

---

## The F-key bar (modifiers)

The bottom F-key bar has **four views**, cycled by `Ctrl+P`:

| Modifier | What each F-key does |
| --- | --- |
| **None** (default) | F1 Help, F2 User, F3 View, F4 Edit, F5 Copy, F6 Move/Rename, F7 Rename, F8 Delete, F9 Menu, F10 Quit, F11 (empty by default), F12 Screens |
| **Ctrl** | F1 Left panel, F2 Right panel, F3 Sort by Name, F4 Sort by Extension, F5 Sort by Time, F6 Sort by Size, F7 Unsorted, F8 Sort by Creation, F9 Sort by Access, F10 Sort by Description, F11 Sort by Owner, F12 Sort Modes |
| **Alt** | F1 Left drive, F2 Right drive, F3 View alt, F4 Edit alt, F5 Print, F6 Make link, F7 Find, F8 History, F9 Video, F10 Tree, F11 View history, F12 Folders history |
| **Shift** | F1 Add to archive, F2 Extract, F3 Archive commands, F9 Save setup, F11 Install dev plugin (only inside a plugin dev dir) |

The cycling is useful when the terminal cannot report live modifier
state (e.g. plain SSH without X11 forwarding). See
[`30_howto_ssh_modifier_keys`](30_howto_ssh_modifier_keys.md) for the
full story.

---

## Quick styling tips

| Goal | Try |
| --- | --- |
| Calm look with subdued blues | Theme = `slate`, set `panel.border = "DarkGray"`. |
| High contrast for a projector | Theme = `classic_blue`, raise `panel.background` to `Black`. |
| Folders pop | In Colors → Color groups, set `file_directory = "Yellow"`. |
| Tame the noisy update badge | Set `dismissed_update_version` in `settings.toml`. |
| Smaller terminal footprint | Toggle "Show bottom F-keys bar" off (`Configuration → Interface`). |

---

## Where to go next

- Full theme TOML schema: [`42_reference_themes`](42_reference_themes.md)
- Every configuration field: [`41_reference_configuration`](41_reference_configuration.md)
- F-key bar reference: [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
