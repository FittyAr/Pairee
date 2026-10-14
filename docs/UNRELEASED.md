## [Unreleased]

### Added
- Theme files have a `[ui]` table of semantic colors (accent, warning, danger, error, success, muted, ...) used by dialogs, the transfer panel, status labels and progress bars; they are also the `ui_*` entries of "Color groups". Themes without it keep today's colors.
- Duplicate finder: "Follow symbolic links" option (files below linked folders are searched; a file reached twice counts once and loops are skipped).
- Plugin API 1.3.0: positional widgets (`ui.Rect`, `Pad`, `Pos`, `Bar`, `Clear`, `Fill`, `widget:area(rect)`, `Layout:split(rect)`, lists of widgets from `peek`), `pairee.cx.tabs` / `tasks` / `yanked` and panel URLs, `File` inherits the fields of `Cha`, and `pairee.fs` on remote URLs of connections open in a panel (trusted plugins; read-only in Secure Mode).
- `pairee --help` (`-h`) lists the options and subcommands, and `pairee plugin|developer|keymap --help` the commands of each, in the configured language.
- Linux and macOS: "Copy attributes" in the Copy / Move dialog also copies extended attributes, and "Copy access control list" copies POSIX ACLs (Linux) and extended ACLs (macOS).
- Interrupted copies to and from SSH servers resume from the part already written, like local ones.
- Tar (plain, .gz, .bz2, .xz) and 7z archives can be changed like zip ones: copy files into them, create folders, edit and delete entries. Files and folders can also be moved into and out of zip, tar and 7z archives.
- `install.sh` installs the aarch64 build on 64-bit ARM Linux, and `install.ps1` the ARM64 build on ARM64 Windows.
- Git: fetch, pull and push now work with SSH remotes (`git@...`), using ssh-agent or your default keys in `~/.ssh`.
- Plugins can be enabled or disabled in the Plugin Manager (`e`); disabled plugins are not loaded.
- Secure Mode can be toggled in the Plugin Manager (`s`); trust, enable and Secure Mode changes apply at once by reloading the plugins.
- Secure Mode can also be switched in the configuration dialog (Plugins tab); OK applies it at once, like the Plugin Manager.
- Plugin settings can be edited in the Plugin Manager (`c` or Enter): a form with a checkbox or field per setting, checked against its type, applied at once.
- Plugin previews in the Quick View can show images, styled text, lists, tables, gauges and split layouts; plugins can prepare previews ahead of time and scroll them with PgUp / PgDn.
- Plugin API 1.2.0: clipboard, persistent plugin state, path / URL / file-information values, image helpers, theme and runtime information (see `docs/api/lua/v1.md`).
- Setting "Overwritten files go to the Recycle Bin" (System tab, `transfer_trash_replaced`, on by default) decides whether a file replaced by a copy or move is sent to the trash first.
- SSH preset passwords are stored in the system keyring; passwords found in `config.toml` are moved there automatically.
- Plugin registry files are checked against minisign signatures. Once Pairee has seen the registry signed, unsigned registry files are refused (`[security] registry_require_signature`, on by default; until then they only show a warning).
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
- Editor: find and replace dialog (`Ctrl+F7` / `Ctrl+H`; find stays on `F7` / `Ctrl+F`) with case sensitive, whole words, regular expression (`$1` groups in the replacement), backward and wrap-around options. Replace goes match by match and "All" replaces every match as one undo step; `Alt+F7` / `Shift+F3` search backward. Matches on screen are highlighted, and patterns and replacements are kept in the history (`Ctrl+Up` / `Ctrl+Down` in the dialog).
- Syntax highlighting in the editor and the viewer, in new `[syntax]` theme colors, for 29 languages (Rust, C, C++, C#, Go, Java, Kotlin, Swift, JavaScript, TypeScript, Python, Ruby, PHP, Lua, shell, PowerShell, batch, HTML, CSS, XML, JSON, YAML, TOML, INI, Markdown, SQL, Makefile, Dockerfile, diff). Settings "Syntax highlighting" and "Highlight files up to" (Editor tab, 2 MiB by default) apply at once to open editors and viewers, and "Save as" with another extension switches the language. Languages are TOML files of regular expressions: put your own in the `syntax` folder of the configuration directory to add languages or replace built-in ones (see the help, "Syntax highlighting definitions").
- Syntax highlighting understands nested languages and matching delimiters: HTML highlights `<script>` as JavaScript and `<style>` as CSS, PHP files are HTML with PHP inside, and Markdown code blocks use the language of their info string; heredocs (shell, PHP, Ruby), Rust raw strings, Lua long brackets and C++ raw strings end at their own delimiter; JavaScript, TypeScript and Ruby regular expressions are told apart from divisions. Definition files can do the same (`${1}` in `end`, `after`, `include`, `host`).
- Your syntax definitions are reloaded when a file in the `syntax` folder changes, or with "Reload syntax highlighting definitions" (command palette); open editors and viewers are highlighted again, and a file with errors is reported and keeps its previous version.
- Branch view (`Ctrl+Alt+X`, `Ctrl+Shift+B` in the Standard preset, Commands menu): every file below the current folder in one flat list with its relative path, read in the background; sorting, filters, selection and file operations work on it, and the action again (or leaving the folder) returns to the normal listing.
- Jump to a frequent folder (`Ctrl+Alt+Z`, `z` in the yazi preset, Commands menu): visited folders ranked by frecency as zoxide does, with a fuzzy filter. `z <words>` on the command line jumps straight to the best match. The ranking is saved in `frecency.toml` only while "Save folders history" is on.
- Compare files (`Ctrl+Alt+C`, Commands menu): a unified or side-by-side diff of the two selected files (or the cursor file of each panel) outside Git; binary files show whether they are identical, with their hashes.
- Duplicate finder (`Ctrl+Alt+U`, Commands menu): searches the selection or the current folder (smallest size and hash algorithm to choose), groups the duplicates with their sizes and wasted space, marks every copy but the newest, oldest or first, deletes or trashes the marked files and shows a file in the panel.

- Connection manager (`Ctrl+Shift+S`, Files → Connect to server...): save, edit, delete and connect SFTP, FTP, FTPS, WebDAV (HTTP / HTTPS) and S3 connections and open them in either panel. Copy and move work between any two places: local ↔ remote and remote ↔ remote (e.g. FTP → S3), with the transfer panel's progress, conflicts, retries and verification.
- Pairee asks for a missing password or key passphrase in a prompt while connecting, instead of failing: SFTP logins, encrypted SSH keys (also for Git fetch, pull and push), and FTP / WebDAV / S3 connections without a stored password or whose password was rejected. Esc cancels.
- FTP: file information comes from `MLST` when the server supports it (one request instead of listing the whole folder).
- FTPS and WebDAV over HTTPS work with self-signed and private-authority certificates: Pairee shows the SHA-256 fingerprint of a certificate the system does not trust and, when you accept it, remembers it for that server; the TLS options of a connection can also name a trusted PEM file (`ca=`) or turn checks off (`insecure=true`, with a warning on every connection).

### Improved
- The frequent-folder list leaves out (and forgets) local folders that no longer exist when it opens, not only when one is chosen.
- The branch view is kept in the session (a restored tab shows it again) and refreshes itself when files change in any subfolder (watched natively, or polled on network folders and SFTP).
- When a plugin copies text of several lines to the clipboard, Pairee says so ("Plugin X copied N lines to the clipboard"), so a command hidden on a later line does not get pasted unnoticed.
- Plugins in Secure Mode can no longer use arguments of an allowed tool to run other programs (`git -c core.sshCommand=…`, `--upload-pack`, `tar --to-command`, `fd --exec`, `rg --pre`, `fzf --preview`, ...); `sed` and editors can no longer be declared.
- Plugins: `pairee.ps.sub` callbacks get who sent each event (`meta.sender`, `meta.trusted`, `meta.from_host`), set by Pairee, and untrusted plugins can no longer publish under Pairee's own event names (`on_cd`, `on_key`, ... and `pairee.*`).
- Git: a pull whose merge conflicts no longer gives up: like `git pull`, it leaves the merge in progress with conflict markers in the files, lists them, and the Git panel title shows "MERGING (N conflicts)"; stage each resolved file with Space and commit, or abort with `X` (also for cherry-picks and reverts).
- Git: merges, cherry-picks and reverts without conflicts are committed at once, like the Git command line (default merge message, original message and author for cherry-picks, "Revert ..." for reverts).
- Git: passphrase prompts of fetch, pull, push and clone go to the window that started them (the same per-state prompt connections use) instead of a process-wide one.
- Git errors that were still in English (no working directory, unsupported merge, branch not found, rebase failures, no remote) are translated.
- A viewer still opening a file applies highlighting settings changed in the meantime once the file is loaded.
- A dialog opened from another one now shows over it instead of hiding it: the file mask and folder tree of the Copy / Move dialog, the Git prompts (commit, checkout, diff, branch and tag names, confirmations, remotes) over the Git panel, and the development plugin picker over the Plugin Manager. Closing it returns to the dialog underneath.
- The screens list (F12) opened over dialogs brings all of them back when closed, not only the top one.
- The color groups and file highlighting dialogs open over the configuration dialog instead of replacing it, and what they save is no longer undone by pressing OK in the configuration dialog.
- The Git diff view can show the changes side by side (`Tab`).
- Git text prompts (commit message, branch / tag / stash names, add remote, clone) accept pasted text, and keys of the `[list]` keymap no longer turn typed letters into list moves there.
- Files added to a zip, tar or 7z archive keep their permissions and modification time, and extracting zip and 7z archives restores the stored Unix permissions.
- Moving files or folders within one drive is instant: they are renamed instead of copied and deleted. Moves to another drive, moves with "Verify after copy" and filtered moves still copy.
- Copies and moves to and from SSH servers now ask (or follow the setting) when a file exists, retry failed files, verify copies, keep times and permissions, and write each file under a temporary name first.
- Direct I/O ("Disable write cache") now bypasses the cache for every whole block of a file, not only for files whose size is a multiple of 4 KiB.
- Queued transfers start at once instead of after a short delay, and an idle transfer engine no longer wakes up five times a second.

- Text in the plugin manager, shortcuts list, file associations and undo labels is cut by display width, so wide characters stay aligned.
- Theme colors and highlighting rules are prepared once when they change instead of on every frame.
- Pictures in the quick view, plugin previews and the viewer are resized once per size instead of on every redraw.

### Changed
- Renamed files are colored in the panels as in the Git panel.
- Sizes in dialogs, the transfer panel and the viewer status line use binary units (KiB, MiB, GiB), matching the 1024-byte steps they are computed with.
- The log file `app.log` is appended to instead of truncated on every start, and rotates to `app.log.1` past 5 MiB; two Pairee windows no longer overwrite each other's log.
- Release builds log at `info` level instead of `debug` (`RUST_LOG=debug` restores the detail); errors while loading the configuration or running a subcommand are now logged too.
- `install.sh` requires `minisign` and a valid release signature; `--insecure-skip-signature` accepts the download on its SHA-256 checksum alone.
- `install.ps1` always verifies the release signature, with the official minisign build it downloads itself (pinned version and SHA-256); `-InsecureSkipSignature` accepts the download on its SHA-256 checksum alone. Both install scripts can also just download and verify (`--verify-only` / `-VerifyOnly`).
- Release archives are named after the platform instead of the Rust target triple: `pairee-X.Y.Z-linux-x64-musl.tar.gz`, `pairee-X.Y.Z-macos-arm64.tar.gz`, `pairee-X.Y.Z-windows-x64.zip`... Until 0.12.0 each archive is also published under its old name, so Pairee 0.10 and older can still update itself.
- Git: pull follows the branch's configured upstream instead of always using the same-named remote branch.
- Git: aborting a merge keeps the local changes you had in other files, like `git merge --abort`.
- Untrusted plugins can only fire navigation, view, sort, selection and help actions (and their own commands) with `pairee.emit`.
- In Secure Mode plugins can no longer write to Pairee's config folder, and commands they run only get locale, terminal and colour variables from them.
- Plugin `min_pairee` versions are read as semver (pre-releases accepted); an unreadable version stops the plugin from loading.
- Plugin settings are converted with the type declared in `settings_schema`.
- "Verify after copy" reads every copy back from the disk and compares it with the source; a file that does not match is retried and, if it still differs, reported as failed. Files copied without verification are no longer marked as verified in the job log.
- Terminal screens (`command &`) and apply-command run on Pairee's own pseudo-terminal layer (ConPTY on Windows, `posix_openpt` on Unix) instead of `portable-pty`, which also drops the unmaintained `serial` crate (RUSTSEC-2017-0008) and a second copy of `nix`, `bitflags`, `thiserror` and `winreg`. A terminal screen's process now sees the size of Pairee's window and is told when it changes.
- `cargo deny` now fails on duplicate versions of shipped crates; the few that only an upstream release can remove are listed in `deny.toml` with the condition to drop them.

- The screens list and the command-line screen indicator are translated.
- The transfer panel (conflict dialog, jobs list, tabs, options, statistics), the yazi sort and view key boards and the development plugin picker are translated.
- yazi preset: `z` jumps to a frequent folder (as zoxide in yazi); finding files stays on `s` / `S`.
- Keymap problem reports are translated; unused translation keys were removed and a few Spanish texts translated.

- SSH presets became connections of any protocol: `[[ssh_presets]]` in `config.toml` is read as SFTP connections and written back as `[[connections]]`; their keyring entries move from `ssh-preset:<name>` to `connection:<name>`. `[security] ssh_passwords_in_config` is now `passwords_in_config` (the old name still works). Saved sessions with SSH tabs still reopen.
- Building Pairee from source needs Rust 1.96 or newer (was 1.93), the version the latest dependencies require.

### Deprecated

### Removed
- `run.bat` and `run.sh`: their developer menu is now `cargo xtask dev` (also `cargo xtask dev <action>`, e.g. `cargo xtask dev clippy`).

- Copying a folder into itself or a file onto itself showed the untranslated message key instead of the error; the help index shows translated titles for every page (it showed file names such as "Ssh Sftp").
### Fixed
- Git: a repository whose status cannot be read (for example a corrupt index) no longer looks clean: the Git panel shows the error in the Status tab, the panel header says "status unavailable", and Commit reports the error instead of "No changes".
- Git: pressing Esc while a pull, fetch or push is finishing no longer claims it was cancelled when it had already completed (a pull may have merged): the popup shows "Cancelling…" and then the real outcome. A second Esc hides the popup.
- Git: deleting a local branch that is not fully merged (into its upstream, or HEAD) now asks a second time, saying its unmerged commits will be lost, instead of deleting it straight away.
- Cancelling a "run command on each file" job stops the running command instead of waiting for it to finish.
- A command running in a terminal screen that does not read its input no longer freezes Pairee when you type into it or stop it; on Linux and macOS, stopping it (F8) hangs up the jobs it started too, and pressing F8 again kills them.
- Windows: closing the console window, logging off or shutting down saves the session and restores the terminal before Pairee ends (it was killed at once); Ctrl+Break quits cleanly like Ctrl+C.
- Linux and macOS: a second termination signal is handled too, and resizing the terminal no longer repaints twice. Linux: reading the held modifier keys from X11 no longer reconnects to the display every 50 ms.
- Two Pairee windows open at once no longer undo each other's saved settings, keybindings, history and folder frecency: each saves only what it changed, on top of what the other saved (under a file lock). The session is written whole by the last one to quit.
- `install.ps1 -Uninstall` run through `irm | iex` no longer closes the PowerShell window, and installing or uninstalling no longer rewrites user `PATH` entries such as `%USERPROFILE%\bin` as fixed paths; uninstall removes every copy of the Pairee entry.
- The ARM64 Windows installer refuses to install on x64 Windows, where its program cannot run.
- `install.sh uninstall` also offers to remove the log and the plugins' data, not only the configuration.
- Folder sizes and the disk usage view are marked incomplete again when some entries of a folder cannot be read, and copies report such folders instead of silently leaving those entries out.
- Previews drawn with `pairee.ui` are shown in the Quick View, and plugin commands, previews and hooks can use dialogs, `pairee.sync` and other calls that wait; waiting for the user no longer counts against a plugin's 10-second limit.
- A keyring that could not be read at startup (locked, temporarily unavailable) no longer makes the next save delete the stored connection password; it is read again when connecting.
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
- Uninstalling a plugin in the Plugin Manager (`d` / Del) asks for confirmation first.
- PgUp / PgDn in the quick view scroll to the real end of the text whatever the panel height, and a plugin preview scrolls by its own rows (images, which are scaled to fit, no longer keep a hidden scroll position).
- "Select folders" applies the same way everywhere: it decides whether masks (`+` / `-`), Select all, Invert selection, Shift+click and right-drag ranges and visual mode take folders, while Insert, Ctrl+click and a right click always toggle the entry under the cursor (see the help).
- One-line text fields in dialogs scroll sideways so the cursor stays visible on long paths, and a click in a field (scrolled or holding wide characters such as CJK) puts the cursor where it was clicked.
