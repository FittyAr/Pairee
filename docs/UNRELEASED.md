## [Unreleased]

### Added

### Improved

### Changed

- The official plugin registry is now signed, so Pairee refuses registry files without a valid signature from the first download (`REGISTRY_SIGNED`); before, a fresh installation only warned. The registry is re-signed weekly.
- Plugins: child processes start with no stdin by default (they used to inherit the terminal) and `Command:cwd` needs write access in Secure Mode; `Command:timeout` is new. The Secure Mode workspace is the active panel's local folder, and the plugin data folder is `plugin-data/<name>` (migrated from `<name>.pairee`).
- Release files no longer carry a second copy named with the Rust target triple (`pairee-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz` and so on): only the platform names (`pairee-X.Y.Z-linux-x64-musl.tar.gz`) remain. 0.11.0 was the last release with both, so Pairee 0.10 or older must update to 0.11.0 (or reinstall) before it can update further. The release build jobs are named by platform too.

### Deprecated

### Removed

### Fixed

- Plugins: Secure Mode no longer lets `Command:cwd` or `tar`/`git`/`zip` path arguments write into the config or other plugins; uninstalling clears the plugin's trust and data; an unreachable blocklist or a corrupt `plugins.lock` is an error instead of being ignored; installs and `install_local` are staged and swapped atomically; reloads cannot register a stale plugin; `developer package` no longer rewrites the source manifest.
- Transfers and remotes: 7z extraction no longer hangs on stderr output, FTP/S3 paths with spaces or accents and WebDAV absolute links open correctly, SFTP write errors are reported, copying a folder into a same-named panel folder nests, `descript.ion` handles names with quotes, archive entries with Windows-invalid names are refused, and the admin listing asks for the sudo password.
- Git: Enter on a stash applies it (`p` pops), renames show as renames, `abort_merge` needs a pending operation, and discarding a dangling symlink works.
- Updater and installers: the download has an idle timeout instead of 120 s total, a second update on Linux no longer writes `pairee (deleted)`, and the installers clear old settings only after the download is verified.
- Interface: dragging files onto the command line quotes paths correctly on Windows, the history keys no longer erase the line, renaming rejects paths, credential prompts no longer steal typed keys, `cd a && b` goes to the shell, and the info panel and several messages are translated.
