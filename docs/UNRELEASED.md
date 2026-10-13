## [Unreleased]

### Added
- `pairee --help` (`-h`) lists the options and subcommands, and `pairee plugin|developer|keymap --help` the commands of each, in the configured language.
- Linux and macOS: "Copy attributes" in the Copy / Move dialog also copies extended attributes, and "Copy access control list" copies POSIX ACLs (Linux).
- Interrupted copies to and from SSH servers resume from the part already written, like local ones.
- Tar (plain, .gz, .bz2, .xz) and 7z archives can be changed like zip ones: copy files into them, create folders, edit and delete entries. Files and folders can also be moved into and out of zip, tar and 7z archives.
- `install.sh` installs the aarch64 build on 64-bit ARM Linux, and `install.ps1` the ARM64 build on ARM64 Windows.
- Git: fetch, pull and push now work with SSH remotes (`git@...`), using ssh-agent or your default keys in `~/.ssh`.
- Plugins can be enabled or disabled in the Plugin Manager (`e`); disabled plugins are not loaded.
- Secure Mode can be toggled in the Plugin Manager (`s`); trust, enable and Secure Mode changes apply at once by reloading the plugins.
- SSH preset passwords are stored in the system keyring; passwords found in `config.toml` are moved there automatically.
- Plugin registry files are checked against minisign signatures; unsigned registry files show a warning and can be refused with `[security] registry_require_signature`.
- The Copy / Move dialog options now work: "Copy files access mode" keeps permissions (Unix) or the access control list (Windows), "Produce sparse files" leaves runs of zeros as holes, "Use copy on write" clones the data instantly where the file system supports it (Btrfs, XFS, APFS, ReFS) and copies normally elsewhere, and "Process multiple destinations" copies to every `;`-separated destination (a move copies to all and moves to the last).
- "Process multiple names" in the Make folder dialog creates every `;`-separated name.
- Copies resume where they stopped: a copy interrupted by a crash, a power cut or a failed attempt continues from the part already written (after checking it matches the source).
- When a copy overwrites a file, the old file goes to the trash, so undoing the copy brings it back.

- Terminal screens of `command &` can be closed (F10, the close box or Delete in the screens list), their process stopped (F8), scrolled back (Shift+arrows, Shift+PageUp/PageDown, mouse wheel) and typed into.
- Setting "Lines kept per terminal screen" (System tab) bounds the output a terminal screen keeps.
- Command line editing: cursor keys, Home/End, Delete, Ctrl+W to delete a word and Tab to complete file and folder names.
- A terminal smaller than 24×8 shows a "terminal too small" notice instead of a squeezed layout.
- The viewer's key bar shows the encoding key (F8).
- Help page with the keys of the Vi layer shared by the Neovim and yazi presets.

### Improved
- Files added to a zip, tar or 7z archive keep their permissions and modification time, and extracting zip and 7z archives restores the stored Unix permissions.
- Moving files or folders within one drive is instant: they are renamed instead of copied and deleted. Moves to another drive, moves with "Verify after copy" and filtered moves still copy.
- Copies and moves to and from SSH servers now ask (or follow the setting) when a file exists, retry failed files, verify copies, keep times and permissions, and write each file under a temporary name first.
- Direct I/O ("Disable write cache") now bypasses the cache for every whole block of a file, not only for files whose size is a multiple of 4 KiB.
- Queued transfers start at once instead of after a short delay, and an idle transfer engine no longer wakes up five times a second.

- Text in the plugin manager, shortcuts list, file associations and undo labels is cut by display width, so wide characters stay aligned.
- Theme colors and highlighting rules are prepared once when they change instead of on every frame.

### Changed
- Sizes in dialogs, the transfer panel and the viewer status line use binary units (KiB, MiB, GiB), matching the 1024-byte steps they are computed with.
- The log file `app.log` is appended to instead of truncated on every start, and rotates to `app.log.1` past 5 MiB; two Pairee windows no longer overwrite each other's log.
- Release builds log at `info` level instead of `debug` (`RUST_LOG=debug` restores the detail); errors while loading the configuration or running a subcommand are now logged too.
- `install.sh` requires `minisign` and a valid release signature; `--insecure-skip-signature` accepts the download on its SHA-256 checksum alone.
- Git: pull follows the branch's configured upstream instead of always using the same-named remote branch.
- Git: aborting a merge keeps the local changes you had in other files, like `git merge --abort`.
- Untrusted plugins can only fire navigation, view, sort, selection and help actions (and their own commands) with `pairee.emit`.
- In Secure Mode plugins can no longer write to Pairee's config folder, and commands they run only get locale, terminal and colour variables from them.
- Plugin `min_pairee` versions are read as semver (pre-releases accepted); an unreadable version stops the plugin from loading.
- Plugin settings are converted with the type declared in `settings_schema`.
- "Verify after copy" reads every copy back from the disk and compares it with the source; a file that does not match is retried and, if it still differs, reported as failed. Files copied without verification are no longer marked as verified in the job log.

- The screens list and the command-line screen indicator are translated.
- Keymap problem reports are translated; unused translation keys were removed and a few Spanish texts translated.

### Deprecated

### Removed

### Fixed
- Removing or renaming an SSH preset now deletes its old password from the system keyring.
- Starting Pairee with a plugin under development whose folder was gone deleted the saved SSH preset passwords from the system keyring.
- `pairee plugin install <name>` failed for every plugin: the name was sent to the registry with a `.pairee` suffix that plugin names cannot have.
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
- Copying or moving a file onto itself (both panels on the same folder, "Overwrite") emptied or deleted it. It is now refused with an error.
- Overwriting a file was not safe: a cancelled or failed copy left a cut-down file and the original was lost. Copies are now written to `<name>.pairee-part`, flushed to disk and only then put in place; cancelling keeps the original.
- Copying a folder into one of its own subfolders nested copies until the path got too long. It is now refused, and loops of links are detected when links are followed.
- Copying a link changed the permissions and times of the file it pointed to; relative links became absolute (and broke when moved); and replacing a folder with a link could delete the folder. Links are now copied as they are and a folder is never replaced by a link.
- A move reported success even when the source could not be deleted (for example a read-only file on Windows); it is now reported as failed.
- Synchronizing folders with "follow links" could delete files outside the synchronized folders through a link; deletions never follow links now.
- A move with "follow links" took the files out of linked folders outside the moved tree; moves now move links as links.
- Deleting links to folders and junctions failed on Windows.
- Unreadable folders were silently left out of copies, moves, folder imports into archives and SSH transfers; they are now reported.
- A link to a folder on an SSH server no longer stops a whole copy: it is skipped with a warning.
- A broken link at the destination was followed and the copy written where it pointed; it is now replaced like any other file.
- Progress could pass 100 % when files were retried.
- The low disk space warning checked a destination folder that did not exist yet.
- Alternate data streams were copied after the read-only attribute was set, so they were lost; the "copy streams" option now controls them on its own, and long paths work for attributes, streams and permissions on Windows.
- The transfer speed kept being reported after a job ended.
- Find files with "containing text" stopped reading a file at the first byte that was not valid UTF-8, so text in Latin-1, UTF-16 or binary files was missed; files are now decoded like the viewer does (binary files are searched byte by byte), and closing the search stops it at once.
- Adding files to a zip archive changed the archive's permissions to owner-only, and entries over 4 GiB failed; compressing to zip also failed on files over 4 GiB and stopped silently at unreadable folders.

- The plugin manager no longer crashes when a registry plugin has a non-ASCII name or author.
- Plugin manager descriptions no longer freeze the app in a very narrow terminal.
- The panels now use the "Files highlighting" rules from Options instead of always the built-in ones.
- Output of a background command no longer goes to the wrong screen, or gets lost, after an earlier screen is closed.
- `cd` on the command line accepts quoted names (as drag & drop inserts them), `~/folder`, `cd -`, relative paths and SFTP or archive panels, and records the folder history.
- Typed text stays visible on the command line when the prompt is long.
- A double click in a long panel opens the entry that was clicked: the panel scrolls only when the cursor leaves the view, as in Far and Midnight Commander.
- The help window no longer crashes in a very small terminal, and its texts are translated.
- Missing texts for the plugin developer install messages and the video mode hint.
