## [Unreleased]

### Added
- `install.sh` installs the aarch64 build on 64-bit ARM Linux, and `install.ps1` the ARM64 build on ARM64 Windows.
- Git: fetch, pull and push now work with SSH remotes (`git@...`), using ssh-agent or your default keys in `~/.ssh`.
- Plugins can be enabled or disabled in the Plugin Manager (`e`); disabled plugins are not loaded.
- Secure Mode can be toggled in the Plugin Manager (`s`); trust, enable and Secure Mode changes apply at once by reloading the plugins.
- SSH preset passwords are stored in the system keyring; passwords found in `config.toml` are moved there automatically.
- Plugin registry files are checked against minisign signatures; unsigned registry files show a warning and can be refused with `[security] registry_require_signature`.

### Improved

### Changed
- The log file `app.log` is appended to instead of truncated on every start, and rotates to `app.log.1` past 5 MiB; two Pairee windows no longer overwrite each other's log.
- Release builds log at `info` level instead of `debug` (`RUST_LOG=debug` restores the detail); errors while loading the configuration or running a subcommand are now logged too.
- `install.sh` requires `minisign` and a valid release signature; `--insecure-skip-signature` accepts the download on its SHA-256 checksum alone.
- Git: pull follows the branch's configured upstream instead of always using the same-named remote branch.
- Git: aborting a merge keeps the local changes you had in other files, like `git merge --abort`.
- Untrusted plugins can only fire navigation, view, sort, selection and help actions (and their own commands) with `pairee.emit`.
- In Secure Mode plugins can no longer write to Pairee's config folder, and commands they run only get locale, terminal and colour variables from them.
- Plugin `min_pairee` versions are read as semver (pre-releases accepted); an unreadable version stops the plugin from loading.
- Plugin settings are converted with the type declared in `settings_schema`.

### Deprecated

### Removed

### Fixed
- Git: pull and fast-forward merges no longer discard uncommitted changes; they stop with an error when your local changes would be overwritten.
- Git: committing after a merge records both parents and ends the merge; committing after a cherry-pick or revert clears its in-progress state.
- Git: a push the remote rejects is reported as an error instead of a success.
- Git: a wrong or passphrase-protected SSH key no longer makes authentication retry forever.
- Git: rebase skips commits that are already applied instead of aborting.
- Windows self-update (zip install): the new `pairee.exe` is put in place while Pairee runs, and the previous one is restored if that fails, instead of a helper script that waited 2 seconds and copied without checking errors. Restart Pairee to use the new version.
- The updater removes the files earlier updates left behind (the replaced `pairee.exe`, the downloaded installer).
- `install.sh` honours `XDG_CONFIG_HOME`, removes its temporary files when interrupted, replaces a running `pairee` without "Text file busy", and no longer cancels a non-interactive update when `/dev/tty` cannot be opened.
- The MSIX package no longer replaces the minimum Windows version it declares with the Pairee version.
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
