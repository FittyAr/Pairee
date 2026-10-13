## [Unreleased]

### Added
- `install.sh` installs the aarch64 build on 64-bit ARM Linux, and `install.ps1` the ARM64 build on ARM64 Windows.
- Git: fetch, pull and push now work with SSH remotes (`git@...`), using ssh-agent or your default keys in `~/.ssh`.

### Improved

### Changed
- The log file `app.log` is appended to instead of truncated on every start, and rotates to `app.log.1` past 5 MiB; two Pairee windows no longer overwrite each other's log.
- Release builds log at `info` level instead of `debug` (`RUST_LOG=debug` restores the detail); errors while loading the configuration or running a subcommand are now logged too.
- `install.sh` requires `minisign` and a valid release signature; `--insecure-skip-signature` accepts the download on its SHA-256 checksum alone.
- Git: pull follows the branch's configured upstream instead of always using the same-named remote branch.
- Git: aborting a merge keeps the local changes you had in other files, like `git merge --abort`.

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
