## [Unreleased]

### Added

- Interactive dialog for file associations enabling navigation, addition, editing, and deletion, with clear visual prompts and helper hints on keys to use.
- Expanded Git support with comprehensive backend APIs for individual file staging, unified diffs, remote syncing (fetch, pull, push), advanced branch management, stashing, resets, merges, and repository clone/initialization.
- New Git dashboard TUI integration with an interactive 4-tab panel (Status, Log, Branches, and Stash).
- Unified diff viewer modal with syntax-colored lines for additions, deletions, and hunks.
- Interactive popup dialogs for stash creation, branch creation/renaming, and safe confirm-action dialogs for resets, merges, and stashes.
- New Spanish translation and updated English manual for Git integration reference.
- New `F7` Rename action that prompts only for the new filename (with a live collision warning if a sibling already exists).
- `Rename` command added to the **Top Menu Bar → Files** submenu.
- F-key shortcut bar now reads each slot from the active keybinding resolver, so the bar always shows what each F-key actually does.
- `Create folder` (MkDir) action added as a default option in the **User Menu** (`F2`), bindable to key `6`. The action opens the same name prompt dialog used everywhere else.

### Improved

- Alt+G Git panel initialization now populates stash data immediately on launch.
- F2-F12 F-key shortcut bar now matches the actual action each key triggers: F2 = User Menu, F9 = Top Menu, F7 = Rename, F11 = empty (when not bound).
- Bottom F-key bar no longer claims `F11 = Plugin` by default — the F11 slot now renders blank until the user explicitly rebinds the key.

### Changed

- Expanded default file association presets to support a wide range of popular formats (text, code, images, audio, video, documents, and web pages).
- F6 dialog renamed from "Rename/Move" to "Move" only — Rename is its own modal now.
- `Make Folder` and `Plugin commands` moved out of the F-key bar into the **Top Menu Bar → Files** submenu so the bar can focus on the most frequent operations.
- The plugin system (`PluginMenu` action) is no longer reachable from `F11`. It is now accessible exclusively via **Top Menu Bar (`F9`) → Files → Plugin commands**. Power users can still rebind `F11` to `plugin_menu` in `keybindings.toml` if they prefer the old layout.

### Removed

- Default keymap no longer binds `F7` to `MkDir` or `F11` to `PluginMenu`. `MkDir` lives in the User Menu (F2) and `PluginMenu` lives under the top menu bar (F9 → Files). Power users can still rebind the keys in `keybindings.toml`.

### Fixed

- Single file copy target path resolution so copying a file to a target destination path no longer creates an extra directory with the file name.
- F-key bar in `keymaps/*.toml` preset files now reflects the new keymap (F7→Rename, no F7→MkDir, no F11→PluginMenu), so users upgrading keep the bar and behavior in sync.
- Outdated doc comment on `Action::MkDir` that still claimed the action was bound to `F7`.

#### Linux-specific bug sweep (2026-07-26)

A systematic audit of every `cfg(unix)` / `cfg(target_os = "linux")` /
`cfg(not(target_os = "windows"))` code path surfaced 31 defects. All of
them are fixed in this release:

**Build / packaging**

- `.cargo/config.toml` no longer hardcodes `OPENSSL_DIR = "/home/deck/local/openssl"`,
  which broke every other developer's fresh `cargo build` since the
  path only existed on one specific machine. We now rely on
  `pkg-config` and the distro's `libssl-dev` / `openssl-devel`.

**Auto-update (`src/update/installer.rs`)**

- Self-update on Linux is now safe under failure: the new binary is
  staged to `<exe>.new`, integrity-checked (must be a valid ELF magic
  or a `#!` shebang), and only then atomically swapped in via
  `rename(2)`. The previous code would silently leave the user with
  no working binary if the copy failed, and unconditionally deleted
  the backup.
- `extract_tar_gz` no longer relies on the tarball being trustworthy
  without any sanity check; combined with the staging change above
  the user is never executed on a corrupt binary.

**Shell injection (`src/fs/apply_cmd.rs`)**

- `apply_command` previously did `cmd.replace("%f", "\"{path}\"")` and
  ran the result through `sh -c`. A file named
  `evil"; rm -rf ~ #.txt` would have escaped the quotes and yielded
  arbitrary command execution. The template is now split on whitespace
  and the program is launched directly via `execvp`-style argv; paths
  with shell metacharacters are now delivered as a single literal
  argv element.

**Network filesystem detection (`src/fs/transfer/network.rs`)**

- `is_lan_path` used to return the first matching `/proc/mounts` line
  for any path, which falsely flagged the entire filesystem as a
  network mount whenever the root mount happened to be NFS / CIFS.
  The function now picks the longest (most specific) network mount
  match, so a path under a local mount on top of an NFS root is
  reported correctly.
- `get_free_space` used to silently return `u64::MAX` (~18 EiB) when
  `df` was missing or its output was unparsable, which led the UI to
  show "free space: 18 EiB" and happily start transfers that would
  fail. The function now propagates a real `Err` with a message
  explaining the missing `df`.

**Process management (`src/app/sys_helpers/process.rs`)**

- `kill_process` used to send `SIGKILL` directly with no grace
  period. The process now receives `SIGTERM` first, the caller waits
  up to 2 seconds for it to exit cleanly, and only then escalates
  to `SIGKILL` if necessary.

**Recycle bin (`src/fs/transfer/worker.rs`)**

- The Linux `send_to_recycle_bin_helper` used to silently fall back to
  `remove_file` / `remove_dir_all` if neither `gio trash` nor
  `trash-put` was installed. On distros without those tools (Fedora
  Server, RHEL minimal, Alpine) this turned "move to trash" into
  data loss with no warning. The helper now returns an explicit error
  asking the user to install a trash tool or to confirm a permanent
  delete.

**Elevated operations (`src/fs/privileges.rs`, `src/app/input_popup/confirm_dialogs.rs`)**

- `acquire_admin_privileges` on Linux was a misleadingly-named
  function: it ran `sudo -v`, which only caches sudo credentials for
  ~5 minutes and does NOT elevate the current process. The previous
  "retry as admin" popup then called `std::fs::create_dir_all` and
  `std::fs::rename` as the unprivileged user, so the operation
  always failed with the same `EACCES` it had failed with before.
  The popup now builds the right `FsOperation` list and dispatches
  it through `run_in_elevated_helper`, which actually re-execs the
  binary under `sudo` and applies the operations with the elevated
  process.

**SSH (`src/fs/ssh.rs`)**

- `delete_recursive` used to `return self.delete_recursive(path)` the
  moment it hit the first subdirectory of a folder, so any sibling
  files were never deleted. The function now snapshots the directory
  listing, unlinks the sibling files first, then recurses into
  every subdirectory, and finally `rmdir`s the parent. The previous
  algorithm also leaked the SFTP lock across recursive calls.
- SSH key auth now distinguishes "key not found on disk" (log a
  warning, fall through to password / agent) from "key rejected by
  the server" (log a warning, fall through to the next method)
  instead of bailing out with a hard error that locked the user out
  even when they had a working password configured.

**Clipboard (`src/app/input_popup/update_popup.rs`)**

- The Linux clipboard copy used to check only that the spawned tool
  (`wl-copy` / `xclip` / `xsel`) was launchable, then returned. If
  the tool launched but exited non-zero (e.g. `wl-copy` with no
  Wayland session), the user got the "command copied" success
  message while the clipboard was still empty. The helper now
  waits for the child and only returns success on a zero exit,
  falling through to the next tool on failure.

**Power actions (`src/fs/transfer/post_action.rs`)**

- `EjectDrive` used to default to `/dev/sdb` when the caller
  passed an empty device path. This could power off an arbitrary
  disk on the user's system. The function now refuses the
  operation with a clear error if the path is empty or the
  device does not exist.
- `systemctl suspend` / `systemctl hibernate` previously
  propagated a bare "io error" on every failure, which left the
  user guessing why their laptop would not sleep. The function
  now collects the command's stderr and explains the usual causes
  (no active logind session, polkit refusal, logind not running).

**Elevated directory listing (`src/fs/list.rs`, `src/main.rs`)**

- `read_directory_as_admin` used to shell out to
  `sudo python3 -c '<script>' <path>`, which silently failed on
  every minimal distro without `python3` and required a sudoers
  entry for `python3` without a TTY. The function now re-execs
  the Pairee binary itself under `sudo` with a dedicated
  `--list-dir-elevated` flag, returning a JSON array of
  `FileEntry`. No external interpreter, no special sudoers entry
  beyond "user can run /usr/bin/pairee as root", and the path is
  passed as a separate argv element (no shell interpolation).

**Search (`src/fs/search.rs`)**

- Recursive file search no longer loops forever on a symlink
  cycle (e.g. a directory containing a `loop` symlink that points
  back to the directory itself). It now keeps a `Vec<PathBuf>` of
  the canonicalised directories it has already recursed into and
  short-circuits any second visit.

**Standalone terminal launcher (`src/terminal/standalone.rs`)**

- The candidate list of terminals used to start with X11-only
  ones (`gnome-terminal`, `konsole`, `xfce4-terminal`,
  `x-terminal-emulator`, `xterm`). On a pure Wayland session
  (Fedora 40+, Ubuntu 24+) most of those would not even spawn,
  which made `--standalone` silently fall back to running in the
  current terminal. The launcher now checks `$WAYLAND_DISPLAY` /
  `$XDG_SESSION_TYPE` and prioritises Wayland-native terminals
  (`foot`, `wezterm`, `kitty`, `alacritty`) on a Wayland session.
- The argv-stripping logic was also rewritten to be position-
  independent: it now filters out both argv[0] and the
  `--standalone` flag in a single pass, so the child process
  always sees the same arguments the parent had minus the flag.

**Update detection (`src/update/detect.rs`, `src/update/checker.rs`)**

- The Linux install-method detector used `Command::output()` with
  no timeout, so a stuck `pacman` DB lock (or a hung `dpkg` /
  `rpm` invocation) would freeze the application at startup. The
  new `run_with_timeout` helper runs the query in a background
  thread and gives up after 2-5 seconds depending on the tool.
- Nix detection used to match any path containing `/nix/` as a
  substring, which produced false positives for users with
  directories like `~/projects/nix-config/`. The detector now
  matches only real Nix-managed paths (`/nix/store/`,
  `/nix/var/nix/profiles/`, `~/.nix-profile/`).
- `build_client` had two cfg arms that were byte-for-byte
  identical (dead code). The split is preserved with a clear
  comment so future platform-specific TLS configuration has a
  documented home.

**Settings (`src/config/settings.rs`)**

- `default_plugins_dev_dir` produced a 3-level path on Windows
  (`<APPDATA>/pairee/config/plugins`) but a 1-level path on
  Unix, violating the AGENTS.md rule "do not add extra levels
  of nested project directories". Both platforms now use
  `<config_dir>/plugins`.

**Filesystem attributes (`src/fs/attrs.rs`)**

- `get_unix_owner_name` used to re-parse `/etc/passwd` on every
  call, which made a directory listing on a box with thousands
  of system users O(N×M). The lookup is now memoised per-uid
  for the lifetime of the process.

**System drives (`src/app/sys_helpers/storage.rs`)**

- The Linux drives list was a hard-coded `["/", "/home", "/media",
  "/mnt", "/tmp"]`. On Fedora Silverblue / RHEL / OpenSUSE
  Tumbleweed, `$HOME` is `/var/home/<user>`, which was not in the
  list. The list now includes `$HOME` (when set and not `/`),
  `/var/home`, and `/usr/home`, with de-duplication.

**Paths (`src/config/paths.rs`)**

- `get_system_share_dir` only checked `/usr/share/pairee`, so
  user-local installs (the default produced by `install.sh`
  without sudo) and `/usr/local/share` distro layouts both
  silently fell back to "no translations, no help, no themes"
  even though the assets were right there in
  `~/.local/share/pairee`. The function now tries
  `/usr/share/pairee`, `/usr/local/share/pairee`,
  `~/.local/share/pairee`, `$XDG_DATA_HOME/pairee`, and the
  Flatpak exports path, in that order.

**Move / rollback (`src/fs/elevated_helper.rs`)**

- `move_operation` used to do `copy_recursive` followed by
  `remove_*` with no rollback. If the post-copy delete failed
  (e.g. permissions changed mid-operation, network filesystem
  hiccup) the user was left with two copies of the data and a
  half-deleted source. The function now removes the just-copied
  destination on failure and reports both the original error and
  the rollback outcome so the user can attempt manual recovery
  if both steps failed.

**Direct I/O (`src/fs/transfer/direct_io.rs`)**

- The Linux `O_DIRECT` open path opened `path` directly while
  the Windows branch opened the post-`to_long_path` `normalized`
  path. The two branches now use the same `normalized` value
  so the primary attempt and the standard fallback open exactly
  the same file.

**Secure wipe (`src/fs/wipe.rs`)**

- The wipe routine used to overwrite only `file_size` bytes and
  relied on that being the on-disk size. If the file had been
  extended by another process between `metadata()` and our
  write, the tail kept its old content. The routine now
  explicitly truncates the file to zero with `set_len(0)`
  *before* starting the overwrite passes.

**Locale string**

- New translation key `error_elevated_helper_failed` was added
  to surface admin-operation failures from the elevated helper
  process.
