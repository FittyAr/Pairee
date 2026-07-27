# 📚 Pairee Help — Documentation Index

> Welcome to **Pairee**, the dual-panel terminal file manager.
> This index groups every help document by its purpose, so you can jump
> straight to what you need.

The icons below mark each document with the **Diátaxis quadrant** it belongs to.
Pick the one that matches what you want to do *right now*:

| You want to… | Quadrant | Go to |
| --- | --- | --- |
| Learn Pairee from scratch, step by step | 🎓 **TUTORIAL** | [`10_tutorial_getting_started`](10_tutorial_getting_started.md) |
| Understand the two-panel layout and screen system | 🎓 **TUTORIAL** | [`11_tutorial_panels_and_screens`](11_tutorial_panels_and_screens.md) |
| Copy, move, delete, wipe, link, or change attributes | 🔧 **HOW-TO** | [`20_howto_file_operations`](20_howto_file_operations.md) |
| Find files, filter the panel, browse history, jump to a hotlist entry | 🔧 **HOW-TO** | [`21_howto_search_filter_history`](21_howto_search_filter_history.md) |
| Compress, extract, or run archive commands | 🔧 **HOW-TO** | [`22_howto_archives`](22_howto_archives.md) |
| Connect to a remote host over SSH/SFTP and transfer files | 🔧 **HOW-TO** | [`23_howto_ssh_sftp`](23_howto_ssh_sftp.md) |
| Manage a Git repository from the dashboard | 🔧 **HOW-TO** | [`24_howto_git_integration`](24_howto_git_integration.md) |
| Change themes, layouts, and color groups | 🔧 **HOW-TO** | [`25_howto_appearance_themes`](25_howto_appearance_themes.md) |
| Switch keymap presets or customise the User Menu (`F2`) | 🔧 **HOW-TO** | [`26_howto_keymaps_user_menu`](26_howto_keymaps_user_menu.md) |
| Map file masks to launch commands | 🔧 **HOW-TO** | [`27_howto_file_associations`](27_howto_file_associations.md) |
| Install, trust, pin, update, or write plugins | 🔧 **HOW-TO** | [`28_howto_plugins`](28_howto_plugins.md) |
| Build from source, install via scripts, update safely | 🔧 **HOW-TO** | [`29_howto_install_build_update`](29_howto_install_build_update.md) |
| Make `Ctrl`/`Alt` shortcuts work over SSH | 🔧 **HOW-TO** | [`30_howto_ssh_modifier_keys`](30_howto_ssh_modifier_keys.md) |
| Look up a key binding or a F-key slot | 📖 **REFERENCE** | [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md) |
| Look up a settings field, tab by tab | 📖 **REFERENCE** | [`41_reference_configuration`](41_reference_configuration.md) |
| Look up the theme TOML schema or color names | 📖 **REFERENCE** | [`42_reference_themes`](42_reference_themes.md) |
| Look up the full `Action` enum or write a keymap | 📖 **REFERENCE** | [`43_reference_actions`](43_reference_actions.md) |
| Look up SSH dialog fields or SFTP operations | 📖 **REFERENCE** | [`44_reference_ssh_fields`](44_reference_ssh_fields.md) |
| Look up the Lua `pairee.*` API for plugin authors | 📖 **REFERENCE** | [`45_reference_plugins_api`](45_reference_plugins_api.md) |
| Understand how the async filesystem and screens work | 💡 **EXPLANATION** | [`50_explanation_architecture`](50_explanation_architecture.md) |
| Understand the plugin trust model and the Lua sandbox | 💡 **EXPLANATION** | [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md) |
| Understand the update system and the 13 install methods | 💡 **EXPLANATION** | [`52_explanation_update_system`](52_explanation_update_system.md) |

---

## How the help popup works

Press **`F1`** anywhere inside Pairee to open this documentation in-app.
The popup has **two tabs**:

- **Core Help** — every `.md` file in this folder, sorted alphabetically.
- **Plugins Help** — the `help/<lang>.md` file inside each installed plugin.

Use **`Up` / `Down`** (or `j` / `k`) to move through the list, **`Enter`**
to open the highlighted document, **`PageUp` / `PageDown`** to scroll long
text, **`Backspace`** to return to the list, and **`Esc`** to close the
popup.

The first file shown is always this index (`00_index.md`).

---

## If you only read one document…

…read [`10_tutorial_getting_started`](10_tutorial_getting_started.md).
It walks you through install → launch → first tour in under ten minutes.
