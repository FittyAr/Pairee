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
   `io`/`os`/`load`, path-bounded `require`).
3. **Trusted plugin** — `StdLib::ALL_SAFE` (io/os/package, no debug) plus
   `pairee.Command` / `fs.spawn`.
4. **Secure Mode** — extra path jail (workspace + config + cache) and a
   process blacklist (shells, interpreters, network tools).
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
| Plugins | Untrusted sandbox; Secure Mode path + spawn blacklist; trust toggle in Plugin Manager | A **trusted** plugin is full user-level code. Typosquatting in the registry. |
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
