# Pairee — short threat model

Scope: local dual-panel TUI on a machine the user already controls. Pairee is
not a multi-tenant service. Goals: keep plugins, remote FS, updates, and
elevation from becoming an easy remote-code or credential leak path.

## Assets

| Asset | Why it matters |
|-------|----------------|
| Local files the user can see in the panels | Accidental wipe/overwrite; plugin write |
| `config.toml` / `keybindings.toml` | Settings, SSH preset secrets, plugin trust |
| Plugin Lua + lockfile | Code that runs in-process |
| GitHub Releases / installer | Binary integrity |
| Elevated helper (admin copy/mkdir) | Privilege boundary |

## Trust zones

1. **Core UI / Transfer Engine** — Rust, same user as the terminal.
2. **Untrusted plugin** — sandboxed Lua (`base/table/string/utf8/math`, no
   `io`/`os`/`load`, path-bounded `require`). `pairee.fs` is **always**
   jailed, independent of Secure Mode: reads are limited to the plugin's own
   directory and its private data directory (`pairee.fs.data_dir()`,
   `<data>/pairee/plugin-data/<plugin>`); writes are limited to the data
   directory and never reach Pairee's config directory.
3. **Trusted plugin** — `StdLib::ALL_SAFE` (io/os/package, no debug) plus
   `pairee.Command` / `fs.spawn`.
4. **Secure Mode** — extra path jail for trusted plugins (workspace + config
   + cache + plugin dir + plugin data dir) and a process blacklist (shells,
   interpreters, network tools). The flag is read once in Rust and captured
   by the `pairee.fs` / `pairee.Command` closures; `pairee._secure_mode` is an
   informational copy only, so overwriting it cannot switch Secure Mode off.

Path checks normalize `.`/`..` lexically and canonicalize the nearest existing
ancestor, so `..` segments in not-yet-existing paths and symlinks inside the
jail cannot escape it; dangling symlinks are rejected.
5. **Remote SSH/SFTP** — another host; credentials live in user config.
6. **Update channel** — GitHub Releases, SHA-256 checked before install.

## What we assume

- The OS user is legitimate. Disk encryption and account lock are out of scope.
- A compromised user account can already replace the binary.
- Terminal escape sequences: we do not treat untrusted file names as trusted
  UI chrome (plugins still must not inject raw CSI into notify titles blindly).

## Controls (today)

| Area | Control | Residual risk |
|------|---------|----------------|
| Plugins | Untrusted sandbox with an always-on `pairee.fs` jail; Secure Mode path + spawn blacklist; trust toggle in Plugin Manager | A **trusted** plugin is full user-level code (Secure Mode does not restrict its `io`/`os`). Typosquatting in the registry. Path checks are check-then-use (a racing local process could swap a path component). |
| Registry install | Plugin name and author must match `[A-Za-z0-9_-]`; `[files]` keys with `..`, absolute, drive-prefixed, `\` or `:` paths are rejected; every file is SHA-256 verified in memory before anything is written | Hashes come from the same registry as the files; a compromised registry can ship matching hashes. |
| User menu (F2) | `{f}` / `{p}` expanded in a single pass with platform shell quoting, so a file name cannot inject a placeholder or close the quotes | On Windows `cmd /c` still expands `%VAR%` inside double quotes (no injection, but the name may be altered). |
| `pairee.Command` | Blocked if untrusted; Secure Mode `is_command_safe` | Blacklist is name-based (`cmd.exe`, `curl`); a renamed binary is not stopped. |
| SSH presets | Stored in local TOML; password field is optional | Passwords in `config.toml` are **not encrypted**. Prefer key files + agent. |
| Auto-update | Background check; SHA-256 of the artifact; user confirms | Compromised GitHub account or MITM after hash fetch is a project-ops issue. |
| Elevated helper | Explicit confirm (`ConfirmRetryAsAdmin`); Windows / Unix privilege APIs | User can approve a destructive op as admin. No extra UAC reason string beyond the dialog. |
| Transfers | Conflict prompt, optional hash verify, cooperative cancel | Verify-after-copy is off by default. |

## Non-goals

- No telemetry (see [PRIVACY.md](./PRIVACY.md)).
- No sandbox for the main binary (it *is* the file manager).
- No macOS notarization / code-sign story in this document.

## Review triggers

Revisit this file when: registry plugins run by default as trusted; update
becomes silent; SSH passwords get a store; or an elevated helper grows a
network path.
