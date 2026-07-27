# 🔧 How-To: Plugins

> **Quadrant: HOW-TO** — *problem-oriented, focused on installing and managing plugins.*

Pairee plugins are small **Lua** scripts that extend the file manager
with new commands, file previewers, and lifecycle hooks. They run in a
secure sandbox (see [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md)).

This page covers the **user-side** of plugins. If you want to **write**
a plugin, see [`45_reference_plugins_api`](45_reference_plugins_api.md)
and the [Plugin Developer Guide](https://github.com/FittyAr/Pairee/blob/master/docs/plugin-dev-guide.md).

---

## Open the Plugin Manager

There is no longer a default `F11` binding for the manager. Use:

| Trigger | Steps |
| --- | --- |
| Menu | `F9` → `Files` → `Plugin commands` |
| Hotkey | `Shift+F11` (after enabling dev mode) or assign your own |

The manager has **three tabs**. Use `Tab` to cycle.

---

## Tab 1: Installed

Lists every plugin currently loaded, with version, author, and three
optional badges:

| Badge | Meaning |
| --- | --- |
| `[P]` (**Pinned**) | This version is locked; global update skips it. |
| `[T]` (**Trusted**) | Granted extended permissions (network, raw shell). Untagged plugins run in a strict sandbox. |
| `[▲]` | An update is available in the central registry. |

| Key | Effect |
| --- | --- |
| `t` / `T` | Toggle **trust** for the highlighted plugin. |
| `p` / `P` | Toggle **pin** in `plugins.lock`. |
| `u` | **Update** the highlighted plugin in the background. A toast confirms completion. |
| `U` | **Update all** unpinned plugins in a batch. |
| `Del` / `d` / `D` | **Uninstall** the highlighted plugin. |

---

## Tab 2: Search Registry

Browse and install plugins from the central registry.

| Key | Effect |
| --- | --- |
| `/` | Focus the search input (border turns yellow). |
| Type | Live filter. |
| `Backspace` | Edit. |
| `Enter` | Submit the query against the remote index. |
| `i` / `I` | **Install** the highlighted result. Downloads in the background; a toast confirms. |

The registry is hosted on the
[`FittyAr/Pairee`](https://github.com/FittyAr/Pairee) repository
under the `plugin-registry` orphan branch.

---

## Tab 3: Developer Tools

This tab appears **only when** `plugins_developer_mode = true` in
`Configuration → Language & Plugins`. The full developer flow is
documented in the [Plugin Developer Guide](https://github.com/FittyAr/Pairee/blob/master/docs/plugin-dev-guide.md);
the quick version is:

| # | Option | Effect |
| --- | --- | --- |
| 0 | **Select active plugin** | Modal listing every detected dev plugin (scans `plugins_dev_dir` and both panels for a `manifest.toml`). |
| 1 | **Initialize boilerplate** | Generates `manifest.toml`, `main.lua`, `lang/en.toml`, `help/en.md`, `icon.png`, `screenshots/screenshot1.png` from the `plugin-template` branch. Disables itself after a successful init. |
| 2 | **Audit (Lint)** | Runs manifest and Lua audits (unsafe imports, undocumented calls, etc.). |
| 3 | **Package** | Prepares a local temporary clone of the `plugin-registry` branch, embeds the SHA-256 of every file in the manifest, and updates the master `registry/index.toml`. Auto-assigns the MIT license if none is present. Outputs the local cache path. |
| 4 | **Install local dev plugin** | Copies the active dev plugin into the runtime directory and registers it in `plugins.lock`. |
| 5 | **Submit plugin (PR)** | With a GitHub token: forks `FittyAr/Pairee`, pushes the branch, opens a PR. Without a token: prints the exact `git push` commands. **The token is held in memory only**; it is never written to disk or environment variables. |

---

## Trust and pinning

- **Trust** is per-plugin. Trust a plugin when you have read its
  source and understand what permissions it needs. Until trusted, the
  plugin cannot spawn subprocesses or open network sockets.
- **Pin** is per-version. Pin a plugin when you depend on a specific
  version (e.g. for reproducibility). `U` skips pinned entries.

See [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md)
for the full security model.

---

## Where to go next

- Plugin API reference: [`45_reference_plugins_api`](45_reference_plugins_api.md)
- Sandbox and trust model: [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md)
- Plugin developer guide: https://github.com/FittyAr/Pairee/blob/master/docs/plugin-dev-guide.md
