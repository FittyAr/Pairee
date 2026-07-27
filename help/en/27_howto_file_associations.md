# 🔧 How-To: File Associations

> **Quadrant: HOW-TO** — *problem-oriented.*

File associations let you map a **glob mask** (e.g. `*.rs`, `*.{jpg,png}`)
to a **launch command**. When you press `Enter` on a file whose name
matches a mask, Pairee runs the mapped command. You can also define a
separate **viewer command** used for `F3`.

> For one-off commands, use the **Apply command** dialog (`Ctrl+G`) —
> see [`20_howto_file_operations`](20_howto_file_operations.md).
> For repeatable per-extension behaviour, use the File Associations
> editor.

---

## Open the editor

There are two ways:

1. **Top menu**: `F9` → `Commands` → `File associations`.
2. **Command line** (inside Pairee): type `associations` and press `Enter`.

The editor opens as a popup with a single-column list of rules. If no
rules exist yet, the list is empty and the bottom hint shows
`[A] Add  [Esc] Close`.

---

## Add a new association

1. Press `A`, `a`, or `Insert`.
2. Pairee walks you through three input fields, in order:

   | Step | Field | Example |
   | --- | --- | --- |
   | 1 | **Mask** | `*.rs` or `*.{jpg,png,jpeg}` |
   | 2 | **Open command** | `code %f` (run VS Code on the file) |
   | 3 | **View command** *(optional)* | `less %f` (used by `F3`) |

   - `%f` is replaced with the **full path** of the file.
   - `%p` is the **containing directory** of the file.
   - `%%` is a literal `%`.

3. After the last field, press `Enter` to commit. The new rule
   appears in the list.

> The new rule is **persisted immediately** to
   `associations.toml` in your config folder. You do not need to save
   manually.

---

## Edit an existing association

1. Highlight the rule.
2. Press `E`, `e`, or `Enter`.
3. The same three-field sequence runs again, with the current values
   pre-filled. Edit and `Enter` to commit.

---

## Delete an association

1. Highlight the rule.
2. Press `D`, `d`, or `Delete`.
3. The rule is removed. (The change is also written to
   `associations.toml` immediately.)

---

## Keyboard reference

| Key | Effect |
| --- | --- |
| `Up` / `Down` | Move the highlight. |
| `A` / `a` / `Insert` | Add a new rule. |
| `E` / `e` / `Enter` | Edit the highlighted rule. |
| `D` / `d` / `Delete` | Delete the highlighted rule. |
| `Esc` | Close the editor (or cancel the current input field). |

---

## Common recipes

| Goal | Mask | Open command |
| --- | --- | --- |
| Open `.md` files in a pager | `*.md` | `less %f` |
| Open `.pdf` in your viewer | `*.pdf` | `zathura %f` *(Linux)* / `start %f` *(Windows)* |
| Open images in `feh` | `*.{jpg,jpeg,png,gif,webp}` | `feh %f` |
| Edit source files in your editor | `*.{rs,go,py,ts,js}` | `code %f` |
| Diff two files | `*.{patch,diff}` | `code --diff %f` |

---

## Interaction with plugins

Pairee also consults **plugins** for association-like behaviour
(see [`45_reference_plugins_api`](45_reference_plugins_api.md)).
When multiple handlers are registered for the same mask, Pairee shows
a selection dialog (unless *Show standard association* is disabled in
`Configuration → Plugins`).

---

## Where to go next

- Apply a one-off command: [`20_howto_file_operations`](20_howto_file_operations.md) (Ctrl+G section)
- Plugin handler chain: [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md)
