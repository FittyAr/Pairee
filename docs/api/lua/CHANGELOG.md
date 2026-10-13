# Lua API Changelog

Semver of the **plugin Lua surface** (`pairee.*`), independent of the Pairee
crate version. Current: **1.2.0** (`pairee._lua_api_version`).

Policy:

- **MAJOR** — remove or rename a stable binding
- **MINOR** — add a binding, field, or documented return shape
- **PATCH** — bugfix with the same signature

## [Unreleased]

### Added

- `pairee.ps.sub` callbacks receive a second argument `meta`
  (`from_host`, `sender`, `trusted`), recorded by the host.
- `on_mouse` hook event: clicks on the panels and the bars, with the button,
  the cell, what was clicked and (for a file row) its panel and 1-based index.
- `pairee.fs.data_dir()` — the plugin's private data directory.
- `[permissions] commands` manifest field (Secure Mode command allowlist,
  shown in the Plugin Manager details).

### Changed

- Secure Mode refuses arguments that make a declared tool run another
  program (`git -c core.sshCommand=...`, `tar --to-command`, `fd --exec`,
  `rg --pre`, ...); `sed` and editors joined the deny list.
- `pairee.ps.pub` from an untrusted plugin refuses reserved event names
  (`on_*`, `pairee.*`) with an error, so it cannot fake host events.
- `pairee.fs` is always path-jailed for untrusted plugins (read: plugin dir + data dir; write: data dir only).
- `pairee._secure_mode` is informational; the host no longer reads it back.
- Secure Mode: `pairee.Command` / `pairee.fs.spawn` use an allowlist. Only
  commands declared in `manifest.toml` `[permissions] commands = [...]`, given
  by bare name and resolved through `PATH`, may run; shells and interpreters
  stay denied even if declared. Plugins that spawn in Secure Mode must add the
  `[permissions]` table.

### Fixed

- `pairee.utils.quote(s, false)` (Windows) now produces a `cmd.exe`-safe token
  (MSVCRT quoting + `^` escaping of `% ! ^ " & | < > ( )`) instead of
  backslash-escaping, which cmd does not understand.

## [1.2.0] — Unreleased (plugin roadmap M1–M4)

### Added

- `pairee.clipboard(text?)` (read: trusted outside Secure Mode only).
- `pairee.utils.uid/gid/user_name/group_name/host_name`.
- `pairee.Error`, `pairee.Path`, `pairee.Url`, `pairee.Cha`; `File.cha`;
  `pairee.fs.cha(path, follow?)`; `pairee.fs.*` accept `Path` / local `Url`.
- `pairee.state.get/set/all/clear` (per-plugin persistent store).
- `preload(job)` and `seek(job)` are called; `job.file` is a `File`,
  `job.area` has `x`, `y`, `w`, `h`.
- `--- @since <api>` and `--- @sync entry|peek|preload|seek` annotations.
- `pairee.ui` widget userdata (`Style`, `Text`, `Row`, `Border`, `Layout`,
  `Constraint`, `Align`, `Wrap`, `Edge`, builders on every widget).
- `pairee.image.show/info/precache`, `pairee.preview_code`,
  `pairee.preview_widget`, `pairee.rt`, `pairee.th`.

### Changed

- `pairee.ui` constructors return userdata instead of plain tables (the
  1.0 call forms and hand-written `{type = ...}` tables still work).
- All plugin methods (`setup`, `entry`, `peek`, hooks) run asynchronously;
  the CPU watchdog only counts time spent executing, not waiting.

### Fixed

- Widgets returned by `peek` are shown (the 1.0 `pairee.ui` tables could not
  be read by the host before).
- Async bindings (dialogs, `sync`, `sleep`, `Command`) work inside `setup`,
  `entry`, `peek` and hook callbacks.

## [1.1.0] — 2026-08-25

### Added

- `File` metadata fields: `mime`, `mtime`, `is_hidden`, `is_exec` (additive; 1.0 plugins keep working).

## [1.0.0] — 2026-08-18

First versioned snapshot of the productive surface.

### Added

- Interactive dialogs: `pairee.confirm`, `pairee.input`, `pairee.which` (real TUI).
- `pairee.cx` (cwd / hovered `File` / selected) filled inside `pairee.sync`.
- `File` userdata (`name`, `path`, `url`, `size`, `is_dir`, `is_symlink`).
- `pairee.fs.mkdir` / `remove` / `rename` / `copy` / `read_dir` / `file`.
- `pairee.Command` builder and streaming `Child`.
- `pairee._lua_api_version` string for runtime feature checks.

See [v1.md](./v1.md) for the full inventory.
