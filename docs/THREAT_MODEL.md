# Pairee — short threat model

Scope: local dual-panel TUI on a machine the user already controls. Pairee is
not a multi-tenant service. Goals: keep plugins, remote FS, updates, and
elevation from becoming an easy remote-code or credential leak path.

## Assets

| Asset | Why it matters |
|-------|----------------|
| Local files the user can see in the panels | Accidental wipe/overwrite; plugin write |
| `config.toml` / `keybindings.toml` / themes | Settings, plugin trust and Secure Mode, keymaps (written atomically, `0600` on Unix) |
| Connection passwords and S3 secret keys (SFTP, FTP/FTPS, WebDAV, S3) | Stored in the OS keyring (Credential Manager / Keychain / Secret Service), not in `config.toml` |
| Plugin Lua + lockfile | Code that runs in-process |
| GitHub Releases / installer | Binary integrity |
| Elevated helper (admin copy/mkdir) | Privilege boundary |

## Trust zones

1. **Core UI / Transfer Engine** — Rust, same user as the terminal.
2. **Untrusted plugin** — sandboxed Lua (`base/table/string/utf8/math`, no
   `io`/`os`/`package`/`load`). `require` loads modules from the plugin's own
   folder only, through the same containment helper and `cap-std` handle as
   `pairee.fs`, with a module cache private to the plugin. `pairee.fs` is
   **always** jailed, independent of Secure Mode: reads are limited to the
   plugin's own directory and its private data directory
   (`pairee.fs.data_dir()`, `<data>/pairee/plugin-data/<plugin>`); writes are
   limited to the data directory and never reach Pairee's config directory.
   `pairee.emit` may only fire an explicit allowlist of non-destructive
   actions (navigation, view, sort, selection, help) and the plugin's own
   commands; everything else is refused in Rust.
   The preview helpers `pairee.image.show` / `image.info` /
   `pairee.preview_code` also accept the file being previewed (the file of
   the running `peek` / `preload` / `seek` call, recorded in Rust): they hand
   it to the host and return an opaque widget (no getters) or the image
   size, never its bytes. `pairee.image.precache` decodes pixels into a file
   the plugin can read, so its source must pass the read jail and its target
   the write jail. `pairee.clipboard` reading is refused (it needs a trusted
   plugin outside Secure Mode); writing is allowed but stripped of control
   characters other than tab/LF and capped at 1 MiB; text of more than
   one line is announced to the user (plugin name and line count). `pairee.state` stores
   at most 256 KiB in a fixed `state.json` of the data directory, written
   atomically. Remote URLs (`sftp://`, `ftp://`, `webdav://`, `s3://`, ...)
   in `pairee.fs` are refused for untrusted plugins.
3. **Trusted plugin** — `StdLib::ALL_SAFE` (io/os/package, no debug) plus
   `pairee.Command` / `fs.spawn`, and `pairee.fs` on remote URLs of
   connections **already open in a panel tab** (looked up by the main loop
   by scheme, host, port and user; the plugin never opens a connection
   or sees a secret): read and write outside Secure Mode, read only in
   Secure Mode.
4. **Secure Mode** — extra path jail for trusted plugins (reads: workspace +
   config + cache + plugin dir + plugin data dir; **writes: workspace + cache
   + plugin data dir only** — Pairee's config directory, i.e. `config.toml`,
   `keybindings.toml`, associations, user menu, `plugins/` and
   `plugins.lock`, is read-only, so a plugin cannot turn Secure Mode off or
   trust itself) and a process **allowlist**: only commands declared in the
   manifest (`[permissions] commands`), given by bare name and resolved
   through absolute `PATH` entries, may run; shells, interpreters, network
   tools and wrappers stay denied even if declared. Environment variables a
   plugin sets on a child are limited to an allowlist (locale, terminal,
   colour: `LANG`, `LC_*`, `TERM`, `NO_COLOR`, `TZ`, ...) and `Command:cwd`
   must be a folder inside the jail. The flag is decided by the host when the
   plugin is loaded (from the config it already parsed, never re-read by the
   plugin) and captured by the `pairee.fs` / `pairee.Command` closures;
   `pairee._secure_mode` is an informational copy only. It **fails closed**:
   when `config.toml` cannot be parsed, plugins run in Secure Mode. Toggling
   Secure Mode, a plugin's trust or whether it is enabled in the Plugin
   Manager reloads every plugin, so the change applies to the running
   session.

Path checks normalize `.`/`..` lexically and canonicalize the nearest existing
ancestor, so `..` segments in not-yet-existing paths and symlinks inside the
jail cannot escape it; dangling symlinks are rejected. The check yields the
jail root it matched; `read`/`write`/`mkdir`/`remove`/`rename`/`copy`/`list`
/`exists` then run through a `cap-std` directory handle of that root, which
resolves every component beneath the handle and refuses symlinks, junctions
or `..` that leave it at use time (no check-then-use window for content).

Every plugin Lua state (trusted or not) has a memory cap (128 MiB untrusted,
512 MiB trusted) and a CPU watchdog. The Rust dispatcher marks each call into
the plugin (`main.lua`, `setup`, `entry`, `peek`, `preload`, `seek`, hook
callbacks) as "in a Lua call" on entry and clears it on exit; an instruction
hook aborts the call once it has run for 10 s. The calls are asynchronous;
the clock runs only while the call's future is being polled (Lua or a
binding executing) and is paused while it waits for a dialog, `pairee.sync`,
a sleep or a child process, so busy work split across many polls still
trips it while waiting for the user does not. Time spent inside C functions
(`string.rep`, `string.find`, ...) counts, because the budget only restarts
when Rust starts a new call. Once tripped the hook checks every instruction,
so `pcall` loops cannot swallow the abort. A single C call that never returns
to the VM cannot be interrupted (memory-heavy ones hit the memory cap).

Values a plugin hands to the host as JSON (`pairee.emit` arguments,
`pairee.ps.pub`, `json_encode`, `pairee.state`) go through one serde-based
converter that only accepts dense lists or string-keyed maps, at most 32
levels deep and 100 000 values, so sparse tables (`{[2^60]=1}`) or cycles
cannot exhaust host memory. Widgets handed to the Quick View (`peek`,
`pairee.preview_widget`) are plain Rust data bounded to 100 000 nodes and
8 MiB of text; images in them only come from `pairee.image.show` (no table
form names a file) and are decoded by the host within the Quick View size
cap and `image` decoder limits (16384 px per side, 256 MiB).

5. **Remote servers (SFTP, FTP/FTPS, WebDAV, S3)** — another host; secrets live in the OS keyring, the rest of the connection in user config. Secrets typed in a credential prompt cross from the UI thread to the background job over an in-process channel only.
6. **Update channel** — GitHub Releases. Asset URLs must start with
   `https://github.com/FittyAr/Pairee/releases/download/`; the `.sha256`
   and `.minisig` assets are mandatory (minisign signature checked against
   the release public key embedded in the binary, trusted comment must name
   the asset); the artifact is downloaded into memory, verified and
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
| Plugins | Untrusted sandbox with an always-on `pairee.fs` jail; Secure Mode path jail (config dir read-only) + spawn allowlist + env allowlist; memory cap + wall-clock CPU watchdog; trust / enable / Secure Mode toggles in the Plugin Manager (applied by reloading plugins); disabled plugins are never loaded | A **trusted** plugin is full user-level code (Secure Mode does not restrict its `io`/`os`, so it can still write the config directory through `io.open`). Typosquatting in the registry. Metadata of listed/`stat`ed entries (`File` fields, `fs.cha`), the files `pairee.image` and `preview_code` open after their check, and `spawn_copy_task` (handed to the transfer engine as plain paths) are still check-then-use. `pairee.ps.pub` events reach every plugin; their sender (`meta.sender` / `meta.trusted`, `meta.from_host` for host hooks) is recorded by Rust, and untrusted plugins cannot publish reserved names (`on_*`, `pairee.*`), but a subscriber that ignores `meta` still acts on any plugin's message. Clipboard writes are open to every plugin: a plugin can still put a multi-line shell command there for the user to paste, but a multi-line write shows "Plugin X copied N lines to the clipboard" first. `pairee.utils.user_name` / `host_name` / `uid` and `pairee.rt` (plugin and data folders, panel preferences) disclose low-sensitivity local facts to every plugin. A single image decode or `string` C call cannot be interrupted by the watchdog (bounded by the size caps). |
| `pairee.emit` / `pairee.app.cd` | The request carries the plugin's name and trust, captured in Rust. Untrusted: explicit allowlist (navigation, view, sort, selection, help, own commands); refused actions show an error. Trusted: any action, run like a key press, so the normal confirmation dialogs and the user's confirmation settings apply. `cd` resolves relative paths, requires an existing local folder and refuses SSH/archive panels | With `confirm_delete = false` a **trusted** plugin can delete the selection without a prompt (same as the user's key). |
| Registry install | Plugin name and author must match `[A-Za-z0-9_-]`; `[files]` keys with `..`, absolute, drive-prefixed, `\` or `:` paths are rejected; `index.toml`, `blocklist.toml` and each plugin `manifest.toml` are checked against `<file>.minisig` (updater's minisign code; trusted comment must name the registry path; pinned release key, configurable); every download is size-limited while streaming and SHA-256 verified in memory before anything is written; files dropped by an update are removed | The registry is not signed yet: a missing signature is accepted with an "unsigned" warning unless `[security] registry_require_signature = true`; until then a compromised registry can ship matching hashes. |
| User menu (F2) / shell commands | `{f}` / `{p}` (and `%f` in apply-command) expanded in a single pass with the shared `shell` quoting module: POSIX single quotes; on Windows MSVCRT quoting plus caret-escaping of every cmd metacharacter (`% ! ^ " & \| < > ( )`), so names are neither split nor `%VAR%`-expanded. cmd is started as `cmd.exe /V:OFF /S /C "<script>"` via `raw_arg` (no Rust `\"` escaping); the PTY path passes the script through an environment variable. Opening a file without association uses `ShellExecuteW` / `xdg-open` / `open` with the path as one argument (no `cmd /c start`) | The template itself is user-written shell code. On the PTY path cmd substring syntax (`%VAR:~0,3%`) is not expanded. |
| `pairee.Command` / `fs.spawn` | Blocked if untrusted. Secure Mode `CommandPolicy` allowlist: bare name only (explicit/relative paths refused), must be declared in `[permissions] commands`, resolved via absolute `PATH` entries (Windows `.exe`/`.com` only, never `.bat`/`.cmd`) and executed by that absolute path; a hard deny list (shells, interpreters, LOLBins, network clients, wrappers; names normalized for case, extension and version suffix) applies to the requested name and to the symlink target. `Command:env` only sets allowlisted locale/terminal/colour variables (no `GIT_*`, `LD_*`, `DYLD_*`, `PAGER`, `EDITOR`, `*_COMMAND`, `BASH_ENV`, `PYTHON*`, `NODE_OPTIONS`, `PERL5*`, `RUBYOPT`, ...); `Command:cwd` must be an existing folder inside the jail. Arguments that make a declared tool run another program are refused per tool (`command_policy/args.rs`: Git config keys naming programs via `-c` / `--config` / `--config-env`, `--upload-pack`, `--exec`, `--template`, `ext::` URLs; `tar --to-command` / `--use-compress-program` / `-I` / `--checkpoint-action`; `zip -TT`; `rg --pre`; `fd --exec`; `fzf --preview` / exec `--bind` actions; `less +cmd`), long options also by abbreviation and short ones inside bundles; `sed` and editors are denied outright, like `ssh`, `rsync` and `find`. Declared commands are shown in the Plugin Manager details | The argument table covers known vectors of common tools only: a declared tool outside it, or a new option of a listed one, may still run code; build tools (`cargo`, `make`, `npm`) run project code by design. A repository's own `.git/config` / `.gitattributes` in the workspace (hooks, textconv drivers) applies to `git`. The user's inherited environment is passed through unchanged. A user-writable directory early in `PATH` can shadow a declared name. The manifest is read at load time; `pairee.fs` cannot rewrite it in Secure Mode, but a trusted plugin still can through `io` (takes effect on next load). |
| Saved connections | Passwords / S3 secret keys live in the OS keyring (`keyring` crate) under `pairee` / `connection:<name>` (entries of older releases, `ssh-preset:<name>`, are moved once); plain-text passwords from older `config.toml` files are migrated and removed from the file. An index entry lists the connections that may have one, so removed or renamed connections lose their entry. A keyring *read error* is not taken for "no password": the entry is never deleted then and is read again when connecting. Without a keyring, passwords are only written to `config.toml` with `[security] passwords_in_config = true` (old name `ssh_passwords_in_config`; otherwise session-only, with a startup warning). URLs built for FTP/WebDAV/S3 logins carry the secret only in memory and are never logged or shown | Anyone with the user's session can read the keyring. FTP (plain) and WebDAV over `http://` send the password in clear text: prefer FTPS, WebDAV over HTTPS, SFTP. Prefer key files + agent for SSH. |
| Credential prompts | Background jobs (SFTP / FTP / WebDAV / S3 logins, Git fetch / pull / push with an encrypted key) ask through `credential_prompt`: a request crosses an in-process channel to a masked prompt; Enter answers, Esc, a closed prompt or quitting answer "none" and the job stops. The secret goes only to the job that asked, is used for that login and is not stored unless the user saves it in the connection; requests from the UI thread itself are refused (no deadlock). Encrypted keys are detected from their header, so passphrases are asked only for keys that need one | A plugin cannot open the prompt, but anything that runs as the user can read the process memory while a secret is in use. |
| Config files | `config.toml`, `keybindings.toml` and theme files that fail to parse are backed up to `.bak`, reported at startup and never overwritten implicitly (only after "Save setup"); `settings.theme` is validated (no separators, `..` or leading dot) before it becomes a path | A file that parses but has wrong values still loads. |
| Auto-update | Background check; URL allowlist; mandatory SHA-256 **and** minisign signature (embedded public key, `file:<asset>` in the signed trusted comment) checked on the in-memory artifact, fail closed; user confirms | Theft of the release secret key (kept offline + GitHub secret). A validly signed *older* asset of the same name pattern is not possible (name includes the version), but no explicit anti-rollback beyond the version check. |
| Install scripts | `curl -fsSL` / `Invoke-WebRequest`; release tag format checked; `.sha256` verified before extraction. `install.sh`: `.minisig` always verified; without `minisign` it stops unless `--insecure-skip-signature` is given. `install.ps1`: `.minisig` verified (mandatory) when `minisign` is installed | `install.ps1` without `minisign`, or `install.sh --insecure-skip-signature`, only checks the same-origin SHA-256. |
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
becomes silent; the plugin registry starts being signed (flip
`registry_require_signature` on by default); or an elevated helper grows a
network path.
