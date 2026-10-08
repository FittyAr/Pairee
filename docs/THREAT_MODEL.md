# Pairee — short threat model

Scope: local dual-panel TUI on a machine the user already controls. Pairee is
not a multi-tenant service. Goals: keep plugins, remote FS, updates, and
elevation from becoming an easy remote-code or credential leak path.

## Assets

| Asset | Why it matters |
|-------|----------------|
| Local files the user can see in the panels | Accidental wipe/overwrite; plugin write |
| `config.toml` / `keybindings.toml` | Settings, SSH preset secrets, plugin trust (written `0600` on Unix) |
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
   + cache + plugin dir + plugin data dir) and a process **allowlist**: only
   commands declared in the manifest (`[permissions] commands`), given by bare
   name and resolved through absolute `PATH` entries, may run; shells,
   interpreters, network tools and wrappers stay denied even if declared. The flag is read once in Rust and captured
   by the `pairee.fs` / `pairee.Command` closures; `pairee._secure_mode` is an
   informational copy only, so overwriting it cannot switch Secure Mode off.

Path checks normalize `.`/`..` lexically and canonicalize the nearest existing
ancestor, so `..` segments in not-yet-existing paths and symlinks inside the
jail cannot escape it; dangling symlinks are rejected.

Every plugin Lua state (trusted or not) has a memory cap (128 MiB untrusted,
512 MiB trusted) and an instruction-count watchdog that aborts Lua code that
runs without returning to Rust for more than 10 s; once tripped it checks
every instruction, so `pcall` loops cannot swallow the abort.

5. **Remote SSH/SFTP** — another host; credentials live in user config.
6. **Update channel** — GitHub Releases. Asset URLs must start with
   `https://github.com/FittyAr/Pairee/releases/download/`; the `.sha256`
   asset is mandatory; the artifact is downloaded into memory, verified and
   installed/extracted from the same verified buffer (no verify→extract
   TOCTOU). Windows helper files live in a random `tempfile` directory.

## What we assume

- The OS user is legitimate. Disk encryption and account lock are out of scope.
- A compromised user account can already replace the binary.
- Terminal escape sequences: we do not treat untrusted file names as trusted
  UI chrome (plugins still must not inject raw CSI into notify titles blindly).

## Controls (today)

| Area | Control | Residual risk |
|------|---------|----------------|
| Plugins | Untrusted sandbox with an always-on `pairee.fs` jail; Secure Mode path jail + spawn allowlist; memory cap + runaway-execution watchdog; trust toggle in Plugin Manager | A **trusted** plugin is full user-level code (Secure Mode does not restrict its `io`/`os`). Typosquatting in the registry. Path checks are check-then-use (a racing local process could swap a path component). |
| Registry install | Plugin name and author must match `[A-Za-z0-9_-]`; `[files]` keys with `..`, absolute, drive-prefixed, `\` or `:` paths are rejected; every file is SHA-256 verified in memory before anything is written | Hashes come from the same registry as the files; a compromised registry can ship matching hashes. |
| User menu (F2) | `{f}` / `{p}` expanded in a single pass with platform shell quoting, so a file name cannot inject a placeholder or close the quotes | On Windows `cmd /c` still expands `%VAR%` inside double quotes (no injection, but the name may be altered). |
| `pairee.Command` / `fs.spawn` | Blocked if untrusted. Secure Mode `CommandPolicy` allowlist: bare name only (explicit/relative paths refused), must be declared in `[permissions] commands`, resolved via absolute `PATH` entries (Windows `.exe`/`.com` only, never `.bat`/`.cmd`) and executed by that absolute path; a hard deny list (shells, interpreters, LOLBins, network clients, wrappers; names normalized for case, extension and version suffix) applies to the requested name and to the symlink target. Declared commands are shown in the Plugin Manager details | A declared tool with its own exec feature (e.g. `git -c core.sshCommand`, `rg --pre`) can still run arbitrary code. A user-writable directory early in `PATH` can shadow a declared name. The manifest is read at load time; a plugin can edit its own manifest (takes effect on next load). |
| SSH presets | Stored in local TOML; password field is optional | Passwords in `config.toml` are **not encrypted**. Prefer key files + agent. |
| Auto-update | Background check; URL allowlist; mandatory SHA-256 checked on the in-memory artifact; user confirms | Hash and artifact come from the same release, so a compromised GitHub account can ship matching hashes (no signature yet). |
| Install scripts | `curl -fsSL` / `Invoke-WebRequest`; release tag format checked; `.sha256` downloaded and verified before extraction | Same-origin hash, as above. |
| 7-Zip helper (Windows) | `7z2601-extra.7z` SHA-256 pinned in the binary and verified in memory before extraction | Updating 7-Zip requires a Pairee release. |
| Archive extraction | Zip/tar/7z/external 7z go through one guard: no `..`/absolute names, never write beneath a pre-existing symlink/junction, never overwrite (existing entries are skipped and reported), 500k-entry / 32 GiB limits; tar hard links and devices are not materialised; external 7z fails closed if listing fails | Check-then-create: a local process racing the extraction could still swap a directory for a link. |
| Secure wipe | `symlink_metadata` first; links are removed, never followed; file opened with `O_NOFOLLOW` / `FILE_FLAG_OPEN_REPARSE_POINT` and re-checked on the handle | SSD wear-levelling / snapshots may keep old data. |
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
