## [Unreleased]

### Added

- Microsoft Store: the package registers the `pairee` command, so the Store installation starts from any terminal, and declares English and Spanish in their regional variants (en-GB, en-AU, en-CA, en-IN, es-MX, es-AR, es-CO, es-CL, es-US) for the Store listings.
- Microsoft Store: every release is now submitted to the Store automatically, both architectures in one bundle with the "What's new" text of each language, so the Store version no longer falls behind GitHub and WinGet.

### Improved

- Connection dialog: the SFTP protocol reads "SFTP (SSH)", so connecting over SSH is easy to find.
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

- SFTP: servers whose `known_hosts` entry holds only some of their host keys (typically just the Ed25519 one that OpenSSH picked) no longer fail with a false "host key does NOT match" warning. Pairee now asks the server for a key type already in `known_hosts`, as OpenSSH does, instead of libssh2's own order (ECDSA first). When the server's key is of a type `known_hosts` lacks, the message says which key to add instead of warning of an attack.
- SFTP and Git over SSH on Windows: Ed25519 and ECDSA host keys and the modern key exchanges (curve25519) work. libssh2 is now built on OpenSSL on Windows too; its Windows backend only knew RSA keys, so a server listed in `known_hosts` by its Ed25519 key was refused.
- Error and info messages show their whole text, word-wrapped in a box that grows with it; long messages (a refused SSH host key, for one) used to be cut at the box's edge.
- Linux installer: `pairee` starts from any new terminal right after `install.sh`, without exporting `PATH` by hand. The installer adds `~/.local/bin` to the `PATH` in the startup files of bash, zsh, fish, ksh and tcsh (on Ubuntu the folder was only picked up at the next login), links `/usr/local/bin/pairee` when run as root, and `uninstall` undoes both.
- Archives: copying out of a 7z (`F5`) or viewing one of its files failed with a checksum error when other entries of the same solid block were passed over.
- Archives: RAR and ISO extraction failed with the 7-Zip Pairee downloads (it cannot read them); an installed 7-Zip is now used first.
- Archives: copying out of an archive and **Extract** follow the conflict setting (overwrite, skip, rename, ask) instead of silently skipping files that already exist.
- Transfers: a job that fails, or ends with files that could not be copied, shows why; before, the minimized transfer bar vanished without a word.
- Transfers: Paste treats the panel folder as the destination, like `F5`: a folder pasted into a folder of the same name nests instead of merging into it (and, after Cut, deleting the source).
- Transfers: **Retry failed** retries each failed file where it was going (keeping its folders) instead of flat in the destination, and remote or archive jobs keep their connection.
- Transfers: starting a queued job (`P`) no longer pauses the running one, which stalled the whole queue, and really moves it ahead of the other queued jobs.
- Copy/Move dialog: it starts from the transfer settings (conflict mode, links, attributes) instead of overriding them; "Rename" can be chosen, and the choices are labelled by what they do ("Overwrite older" read "Append", "Copy link" and "Copy target" were swapped).
- Archives: one item copied out of or into an archive under a name typed in the dialog gets that name instead of a new folder of that name; a renamed copy never takes the name of another extracted file; copies between an archive and a remote panel are refused up front instead of failing file by file.
- Archives: the Copy/Move dialog's filter applies when copying into or out of an archive; `F6` of a folder into a zip with `*.txt` used to pack every file and then delete them all from disk, and now packs and deletes only the matching ones (a filtered move out of an archive likewise removes only what it extracted).
- Transfers: `F6` of a file onto itself (both panels on one folder, name kept) is skipped as "already there" instead of asking to overwrite the file with itself and then failing.
- Transfers: progress totals match the work: moves done as instant renames no longer read "3/0", extracting a few entries counts those entries instead of the whole archive (1 file read 1/5000), and the low-disk-space warning is a log line instead of a skipped file.
- Archives: archive jobs report real paths (the archive file and the folder written to), so the automatic transfer report is saved next to the extracted files instead of in Pairee's working folder, and the "eject" post-action finds the drive.

- Updater: a copy installed from the Microsoft Store is recognized (`WindowsApps` folder) and points to the Store for updates instead of trying to replace itself in a read-only folder.

- Plugins: Secure Mode no longer lets `Command:cwd` or `tar`/`git`/`zip` path arguments write into the config or other plugins; uninstalling clears the plugin's trust and data; an unreachable blocklist or a corrupt `plugins.lock` is an error instead of being ignored; installs and `install_local` are staged and swapped atomically; reloads cannot register a stale plugin; `developer package` no longer rewrites the source manifest.
- Transfers and remotes: 7z extraction no longer hangs on stderr output, FTP/S3 paths with spaces or accents and WebDAV absolute links open correctly, SFTP write errors are reported, copying a folder into a same-named panel folder nests, `descript.ion` handles names with quotes, archive entries with Windows-invalid names are refused, and the admin listing asks for the sudo password.
- Git: Enter on a stash applies it (`p` pops), renames show as renames, `abort_merge` needs a pending operation, and discarding a dangling symlink works.
- Updater and installers: the download has an idle timeout instead of 120 s total, a second update on Linux no longer writes `pairee (deleted)`, and the installers clear old settings only after the download is verified.
- Updater: on macOS, a `pairee` started through a symlink updates the program behind the link instead of replacing the link with a copy.
- Interface: dragging files onto the command line quotes paths correctly on Windows, the history keys no longer erase the line, renaming rejects paths, credential prompts no longer steal typed keys, `cd a && b` goes to the shell, and the info panel and several messages are translated.
