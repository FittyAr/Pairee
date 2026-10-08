# Lua API Changelog

Semver of the **plugin Lua surface** (`pairee.*`), independent of the Pairee
crate version. Current: **1.1.0** (`pairee._lua_api_version`).

Policy:

- **MAJOR** — remove or rename a stable binding
- **MINOR** — add a binding, field, or documented return shape
- **PATCH** — bugfix with the same signature

## [Unreleased]

### Added

- `pairee.fs.data_dir()` — the plugin's private data directory.
- `[permissions] commands` manifest field (Secure Mode command allowlist,
  shown in the Plugin Manager details).

### Changed

- `pairee.fs` is always path-jailed for untrusted plugins (read: plugin dir + data dir; write: data dir only).
- `pairee._secure_mode` is informational; the host no longer reads it back.
- Secure Mode: `pairee.Command` / `pairee.fs.spawn` use an allowlist. Only
  commands declared in `manifest.toml` `[permissions] commands = [...]`, given
  by bare name and resolved through `PATH`, may run; shells and interpreters
  stay denied even if declared. Plugins that spawn in Secure Mode must add the
  `[permissions]` table.

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
