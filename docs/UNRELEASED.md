## [Unreleased]

### Added

- Plugins can be enabled or disabled in the Plugin Manager (`e`); disabled plugins are not loaded.
- Secure Mode can be toggled in the Plugin Manager (`s`); trust, enable and Secure Mode changes apply at once by reloading the plugins.
- SSH preset passwords are stored in the system keyring; passwords found in `config.toml` are moved there automatically.
- Plugin registry files are checked against minisign signatures; unsigned registry files show a warning and can be refused with `[security] registry_require_signature`.

### Improved

### Changed

- Untrusted plugins can only fire navigation, view, sort, selection and help actions (and their own commands) with `pairee.emit`.
- In Secure Mode plugins can no longer write to Pairee's config folder, and commands they run only get locale, terminal and colour variables from them.
- Plugin `min_pairee` versions are read as semver (pre-releases accepted); an unreadable version stops the plugin from loading.
- Plugin settings are converted with the type declared in `settings_schema`.

### Deprecated

### Removed

### Fixed

- A plugin could crash Pairee by passing a sparse table such as `{[2^60] = 1}` to `pairee.emit`.
- Plugins running loops of expensive string functions were never stopped by the execution time limit.
- An unreadable `config.toml` turned Secure Mode off for plugins; plugins now run in Secure Mode until the file is fixed.
- `require` did not work in untrusted plugins.
- Plugin `cd` requests now resolve relative paths, check that the folder exists and leave SSH and archive panels alone.
- Installing, updating and linting plugins from the TUI no longer prints over the screen or waits for terminal input.
- Loading plugins no longer re-reads and may no longer rewrite `config.toml` for every plugin.
- Registry downloads are size-limited, and updating a plugin removes the files the new version no longer ships.
- The Plugin Manager update key (`u`) respects pinned plugins.
- An invalid `keybindings.toml` or theme file is backed up and kept instead of being replaced by the defaults; theme names cannot point outside the themes folder.
- Switching theme no longer overwrites the theme file with the built-in colours.
