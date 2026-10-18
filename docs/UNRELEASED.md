## [Unreleased]

### Added

- Microsoft Store: the package registers the `pairee` command, so the Store installation starts from any terminal, and declares English and Spanish in their regional variants (en-GB, en-AU, en-CA, en-IN, es-MX, es-AR, es-CO, es-CL, es-US) for the Store listings.
- Microsoft Store: every release is now submitted to the Store automatically, both architectures in one bundle with the "What's new" text of each language, so the Store version no longer falls behind GitHub and WinGet.

### Improved

- Archives: RAR, ISO, CAB, WIM, ARJ, LZH, CPIO, RPM, DEB, XAR, DMG, VHD/VHDX, SquashFS, CHM, split `.001` archives and single `.gz`/`.bz2`/`.xz`/`.zst`/`.lzma` files open as folders (view, Quick View, copy out with `F5`, extract) through an installed 7-Zip, which also reads the 7z archives with codecs the built-in reader lacks (Deflate64...). Without a full 7-Zip, a message says to install it.
- Archives: **Extract** unpacks every selected archive (not only the one under the cursor) and says so when there is none; **Test** decompresses every entry instead of only listing them.

### Changed

- Pairee now uses the shared FittyAr folders on every platform. Windows: the program in `%LOCALAPPDATA%\Programs\FittyAr\Pairee` and the settings in `%APPDATA%\FittyAr\Pairee`. Linux: the program in `~/.local/lib/FittyAr/Pairee` (with the `pairee` command still in `~/.local/bin`), the settings in `~/.config/FittyAr/Pairee`, the log in `~/.cache/FittyAr/Pairee` and the plugin data in `~/.local/share/FittyAr/Pairee` (following the `XDG_*` variables). macOS: `~/Library/Application Support/FittyAr/Pairee` and `~/Library/Caches/FittyAr/Pairee`. The folders of earlier versions are moved into place on the first start, by the installers or by the next self-update; nothing is deleted.
- The official plugin registry is now signed, so Pairee refuses registry files without a valid signature from the first download (`REGISTRY_SIGNED`); before, a fresh installation only warned. The registry is re-signed weekly.
- Plugins: child processes start with no stdin by default (they used to inherit the terminal) and `Command:cwd` needs write access in Secure Mode; `Command:timeout` is new. The Secure Mode workspace is the active panel's local folder, and the plugin data folder is `plugin-data/<name>` (migrated from `<name>.pairee`).
- Release files no longer carry a second copy named with the Rust target triple (`pairee-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz` and so on): only the platform names (`pairee-X.Y.Z-linux-x64-musl.tar.gz`) remain. 0.11.0 was the last release with both, so Pairee 0.10 or older must update to 0.11.0 (or reinstall) before it can update further. The release build jobs are named by platform too.

### Deprecated

### Removed

### Fixed

- Archives: copying out of a 7z (`F5`) or viewing one of its files failed with a checksum error when other entries of the same solid block were passed over.
- Archives: RAR and ISO extraction failed with the 7-Zip Pairee downloads (it cannot read them); an installed 7-Zip is now used first.
- Archives: copying out of an archive and **Extract** follow the conflict setting (overwrite, skip, rename, ask) instead of silently skipping files that already exist.
- Transfers: a job that fails, or ends with files that could not be copied, shows why; before, the minimized transfer bar vanished without a word.

- Updater: a copy installed from the Microsoft Store is recognized (`WindowsApps` folder) and points to the Store for updates instead of trying to replace itself in a read-only folder.

- Plugins: Secure Mode no longer lets `Command:cwd` or `tar`/`git`/`zip` path arguments write into the config or other plugins; uninstalling clears the plugin's trust and data; an unreachable blocklist or a corrupt `plugins.lock` is an error instead of being ignored; installs and `install_local` are staged and swapped atomically; reloads cannot register a stale plugin; `developer package` no longer rewrites the source manifest.
- Transfers and remotes: 7z extraction no longer hangs on stderr output, FTP/S3 paths with spaces or accents and WebDAV absolute links open correctly, SFTP write errors are reported, copying a folder into a same-named panel folder nests, `descript.ion` handles names with quotes, archive entries with Windows-invalid names are refused, and the admin listing asks for the sudo password.
- Git: Enter on a stash applies it (`p` pops), renames show as renames, `abort_merge` needs a pending operation, and discarding a dangling symlink works.
- Updater and installers: the download has an idle timeout instead of 120 s total, a second update on Linux no longer writes `pairee (deleted)`, and the installers clear old settings only after the download is verified.
- Updater: on macOS, a `pairee` started through a symlink updates the program behind the link instead of replacing the link with a copy.
- Interface: dragging files onto the command line quotes paths correctly on Windows, the history keys no longer erase the line, renaming rejects paths, credential prompts no longer steal typed keys, `cd a && b` goes to the shell, and the info panel and several messages are translated.
