# 📖 Reference: Theme TOML Schema

> **Quadrant: REFERENCE** — *information-oriented.*

Pairee loads themes from:

- **Windows:** `%APPDATA%\pairee\config\themes\`
- **Linux / macOS:** `~/.config/pairee/themes/`

Each file is a TOML document. The active theme is selected in
`Configuration → Colors → Theme` (the same dialog also has a toggle for
the two bundled themes, `slate` and `classic_blue`).

---

## Minimal example

```toml
[panel]
border          = "Blue"
background      = "Black"
file_selected   = "Yellow"
file_directory  = "Cyan"
file_executable = "Green"

[menu]
background = "Blue"
selected   = "White"
```

That is enough to get a different look. Pairee falls back to a default
value for any key you do not set.

---

## Top-level groups

| Group | Purpose |
| --- | --- |
| `[panel]` | Panel border, background, file-type colours. |
| `[menu]` | Top menu bar colours. |
| `[dialog]` | Popup dialogs (modals, list selectors, prompts). |
| `[viewer]` | Internal file viewer colours. |
| `[editor]` | Internal editor colours. |
| `[status]` | Status line at the bottom of the panel. |
| `[clock]` | Top-right clock widget. |
| `[fkey]` | F-key bar (numbers, text). |
| `[transfer]` | Transfer progress popup. |
| `[highlight]` | Per-extension highlight rules. |

Most groups follow the same colour keys. A few groups have
behaviour-specific keys (e.g. `[fkey]` has `fkey_bg`,
`fkey_num_fg`, `fkey_text_fg`).

---

## Colour values

A colour can be any of:

| Form | Examples |
| --- | --- |
| **Named colour** | `Black`, `Red`, `Green`, `Yellow`, `Blue`, `Magenta`, `Cyan`, `White`, `Gray`, `DarkGray`, `Reset` |
| **Hexadecimal (6-digit)** | `"#FF7700"`, `"#1B2A3F"` |
| **Hexadecimal (RGB triplet)** | `"(255, 119, 0)"` |

Both uppercase and lowercase hex digits are accepted.

> Some terminals (Windows Console, old `cmd.exe`) only render 16 named
> colours. If you specify a hex value and the colour does not appear,
> try a named colour or switch to Windows Terminal / WezTerm.

---

## `[panel]` keys

| Key | Default | Meaning |
| --- | --- | --- |
| `border` | `"Blue"` | Border of the active panel. |
| `border_inactive` | `"DarkGray"` | Border of the passive panel. |
| `background` | `"Black"` | Panel inner background. |
| `selected` | `"Yellow"` | Tag colour for the highlighted row. |
| `file_selected` | `"Yellow"` | Tag colour on selected rows. |
| `file_directory` | `"Cyan"` | Folders. |
| `file_executable` | `"Green"` | Binaries and scripts. |
| `file_symlink` | `"Magenta"` | Symbolic links. |
| `file_archive` | `"Red"` | Archives (zip, tar, 7z, …). |
| `file_image` | `"Yellow"` | Image files. |
| `file_temp` | `"Gray"` | Temp / cache files. |
| `file_hidden` | `"DarkGray"` | Hidden / system files. |

---

## `[menu]` keys

| Key | Default | Meaning |
| --- | --- | --- |
| `background` | `"Blue"` | Top menu bar background. |
| `selected` | `"White"` | Highlighted item text. |
| `unselected` | `"Black"` | Non-highlighted item text. |
| `shortcut` | `"Yellow"` | Mnemonic letter (`&`-accelerator). |
| `separator` | `"DarkGray"` | Separator lines. |

---

## `[dialog]` keys

| Key | Default | Meaning |
| --- | --- | --- |
| `border` | `"Blue"` | Border. |
| `background` | `"Black"` | Body background. |
| `title` | `"Yellow"` | Title text. |
| `text` | `"White"` | Body text. |
| `button` | `"Cyan"` | Button text. |
| `button_active` | `"Yellow"` | Focused button. |
| `input` | `"White"` | Input field text. |
| `input_active` | `"Yellow"` | Focused input field. |

---

## `[viewer]` / `[editor]` keys

| Key | Default | Meaning |
| --- | --- | --- |
| `border` | `"Blue"` | Border. |
| `background` | `"Black"` | Body background. |
| `text` | `"White"` | Plain text. |
| `selection` | `"Yellow"` | Selected text. |
| `cursor` | `"White"` | Caret. |
| `line_number` | `"DarkGray"` | Line-number gutter. |
| `search_hit` | `"Yellow"` | Search match highlight. |
| `hex_byte` | `"Cyan"` | Hex digits. |
| `hex_ascii` | `"Gray"` | ASCII side. |

---

## `[status]` / `[clock]` / `[fkey]` keys

| Group | Key | Default | Meaning |
| --- | --- | --- | --- |
| `status` | `background` | `"Black"` | Status line background. |
| `status` | `text` | `"Gray"` | Status line text. |
| `status` | `selected_count` | `"Yellow"` | Highlighted selection count. |
| `clock` | `background` | `"Black"` | Clock background. |
| `clock` | `text` | `"White"` | Clock digits. |
| `fkey` | `fkey_bg` | `"DarkGray"` | F-key bar background. |
| `fkey` | `fkey_num_fg` | `"Black"` | F-key number foreground. |
| `fkey` | `fkey_text_fg` | `"White"` | F-key label foreground. |

---

## `[transfer]` keys

| Key | Default | Meaning |
| --- | --- | --- |
| `bar` | `"Cyan"` | Progress bar fill. |
| `bar_bg` | `"DarkGray"` | Progress bar background. |
| `text` | `"White"` | File name and stats. |
| `speed` | `"Yellow"` | Speed / ETA. |
| `error` | `"Red"` | Error text. |

---

## `[highlight]` — per-extension rules

The `[highlight]` table lets you map file masks to a colour:

```toml
[highlight]
"*.rs"   = "Yellow"
"*.toml" = "Cyan"
"*.md"   = "Green"
"*.lock" = "DarkGray"
```

A mask follows glob rules (`*`, `?`, `[abc]`). The colour is the same
format as anywhere else (named or hex).

---

## Using a custom theme

1. Save your TOML file in the `themes/` directory.
2. Open `F9` → `Options` → `Configuration` → **Colors**.
3. The first row toggles the bundled theme. Move down to find your
   custom theme in the picker.
4. `Enter` to apply; `F9` again to save.

> Changes are written to `settings.toml` immediately if
> `auto_save_setup = true` (the default).

---

## Troubleshooting

| Symptom | Likely fix |
| --- | --- |
| My theme does not appear in the picker | Verify the file is `.toml` and lives in the right `themes/` directory. |
| Colours look wrong (washed out) | Your terminal may not support true colour. Try Windows Terminal, WezTerm, Alacritty, or `COLORTERM=truecolor` in your environment. |
| A specific slot never changes | The slot is not in the schema above; the key is ignored. Use the **Color groups** dialog in `Configuration → Colors` for the full slot list. |
| Hex colours render as the closest named colour | Your terminal reports `COLORTERM != truecolor`. Pairee falls back gracefully but you lose the precision. |

---

## Where to go next

- Configuration reference: [`41_reference_configuration`](41_reference_configuration.md)
- How to switch theme: [`25_howto_appearance_themes`](25_howto_appearance_themes.md)
