# 💡 Explanation: Plugin Sandbox and Trust Model

> **Quadrant: EXPLANATION** — *understanding-oriented.*

Pairee's plugin system is built around a single principle: **plugins
should not be able to do anything the user has not explicitly
allowed**. This page explains the three concentric rings of trust, and
what each one permits.

---

## 1. The three rings

```
┌──────────────────────────────────────────────────────────┐
│  Ring 0: untrusted (default)                             │
│  - base, table, string, utf8, math                       │
│  - no io, no os, no package, no require                  │
│  - no spawn                                              │
│  - file reads limited to plugin dir + active workspace   │
└──────────────────────────────────────────────────────────┘
                          ▲  user clicks [T] (trust)
                          │
┌──────────────────────────────────────────────────────────┐
│  Ring 1: trusted (per plugin, opt-in)                    │
│  - everything in Ring 0                                  │
│  - fs.spawn (subject to blacklist when Secure Mode)      │
│  - network access through the public API                 │
│  - clipboard, dialogs, notifications                    │
└──────────────────────────────────────────────────────────┘
                          ▲  user enables Secure Mode
                          │
┌──────────────────────────────────────────────────────────┐
│  Ring 2: Secure Mode (global)                            │
│  - 27-command blacklist applied to fs.spawn               │
│  - file access restricted to workspace + config + cache  │
│  - plugins cannot reach arbitrary paths                 │
└──────────────────────────────────────────────────────────┘
```

The user controls each ring explicitly:

- **Trust** is per plugin, toggled by pressing `T` in the Plugin
  Manager. It is sticky: once trusted, the plugin stays trusted
  until you untick it.
- **Secure Mode** is a global flag (`secure_mode = true` in
  `settings.toml` or in `Configuration → Plugins`). It applies to
  every plugin, trusted or not.

---

## 2. What "untrusted" means

A plugin that has **not** been marked trusted runs with:

- The Lua **base**, **table**, **string**, **utf8**, and **math**
  libraries.
- A **bounded** `require` that can only load modules inside the
  plugin's own directory.
- `pairee.*` calls that do not touch the network or spawn
  processes.
- **No** `io`, **no** `os`, **no** `package`, **no** `load`,
  **no** `loadstring`, **no** `dofile`, **no** `loadfile`.
- **No** `pairee.fs.spawn`.
- **No** clipboard / notification / dialog APIs (use the structured
  forms in `pairee.*` for those — they are safe regardless of
  trust).

The intent: a malformed or malicious plugin can read and write
inside its own directory and the active workspace, but cannot
exfiltrate data or take over the system.

---

## 3. What "trusted" means

Pressing `T` on a plugin grants it **Ring 1**:

- `pairee.fs.spawn(cmd, args, opts)` — run a child process.
- `pairee.utils.target_os()` / `target_family()` — informational.
- Full access to `pairee.ui.*` widgets.
- Clipboard via `pairee.clipboard.*`.
- Notifications and structured dialogs.

A trusted plugin can still **not** read or write outside the
workspace + config + cache directories unless Secure Mode is off.

> Trust is per plugin. The Plugin Manager shows a `[T]` badge next
> to trusted plugins so the list is auditable at a glance.

---

## 4. What "Secure Mode" means

When `secure_mode = true`:

- `pairee.fs.spawn` is checked against a **27-command blacklist** of
  network tools, shells, and script runtimes. Examples: `bash`,
  `sh`, `zsh`, `cmd`, `powershell`, `ssh`, `scp`, `sftp`, `nc`,
  `ncat`, `curl`, `wget`, `python`, `python3`, `ruby`, `perl`,
  `node`, `php`, `lua`, `lua5.x`, `awk`, `gawk`, `tclsh`, `wish`,
  `expect`, `socat`, `telnet`. If the command is on the list, the
  spawn is rejected and a log line is written.
- File-system access is **path-restricted** to the active workspace
  + the user's config and cache directories. Reads and writes
  outside those paths are denied.
- Even **trusted** plugins are subject to these rules. Secure Mode
  is the "belt and suspenders" layer.

> The blacklist is intentionally conservative. You can extend it by
> editing the Secure Mode configuration in the source if you need to
> (advanced; consult the developer guide first).

---

## 5. Why pinning matters

Pinning (`P` in the Plugin Manager) writes the plugin's current
version into `plugins.lock`. When you run `U` (update all), pinned
entries are skipped.

This protects you against two scenarios:

1. **Breaking changes.** A new major version of a plugin changes a
   behaviour you depend on. Pin the version you trust; the update
   pass cannot touch it.
2. **Supply chain attack.** A plugin registry is compromised and
   pushes a backdoored version. Pinned entries stay on the version
   you reviewed; you decide when to upgrade.

`plugins.lock` is a plain TOML file in your config folder; you can
edit it by hand for finer control.

---

## 6. Threat model

| Threat | Mitigation |
| --- | --- |
| Malicious plugin in the registry | Trust is opt-in; the Plugin Manager shows the source repo. |
| Plugin bugs that corrupt the workspace | File operations on the workspace are logged to `app.log`. |
| Plugin exfiltrating data over the network | `pairee.fs.spawn` and `pairee.net.*` require trust; Secure Mode adds a command blacklist. |
| Compromised update channel | Updates are gated by SHA-256 (see [`52_explanation_update_system`](52_explanation_update_system.md)). |
| Plugin breaks across Pairee versions | Each plugin declares a `pairee_api_version`; mismatches prevent loading. |

---

## 7. What you can do to harden your setup

1. **Do not trust** plugins you have not read the source of.
2. **Enable Secure Mode** if you handle sensitive data.
3. **Pin** the versions of plugins you depend on.
4. **Review** the Plugin Manager occasionally for `[T]` badges you
   no longer need.
5. **Inspect** `app.log` in the cache folder if a plugin behaves
   oddly.

---

## Where to go next

- Plugin user guide: [`28_howto_plugins`](28_howto_plugins.md)
- Plugin API reference: [`45_reference_plugins_api`](45_reference_plugins_api.md)
- Plugin architecture: https://github.com/FittyAr/Pairee/blob/master/docs/technical/plugin-system-design.md
