# Lua API Changelog

Semver of the **plugin Lua surface** (`pairee.*`), independent of the Pairee
crate version. Current: **1.0.0** (`pairee._lua_api_version`).

Policy:

- **MAJOR** — remove or rename a stable binding
- **MINOR** — add a binding, field, or documented return shape
- **PATCH** — bugfix with the same signature

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
