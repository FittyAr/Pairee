## [Unreleased]

### Added

- `TransferEndpoint` abstraction in `src/fs/transfer/endpoint.rs` that gives the engine a uniform interface to operate on local files and SFTP sessions alike. The `Local` variant wraps the existing `std::fs` / Windows-API surface; the `Ssh` variant wraps `SharedSshClient` and exposes only SFTP-native operations (no shell-out). Phase 1 of the unified-transfer refactor — the engine itself is still local-only and will be re-wired in later phases.
- `xattr = "1.3"` dependency (Unix only) for extended-attribute preservation during transfers.
- `preserve_metadata` is now endpoint-aware (Phase 2 of the unified-transfer refactor). It accepts a source and a destination `TransferEndpoint` and routes timestamp / permission / Unix-xattr preservation through them. Windows ACL and ADS preservation stay in the Local+Windows path. Ssh with capabilities the endpoint does not surface is silently skipped with a `log::warn!` instead of failing the transfer.
- `copy_file_pipelined` is now endpoint-aware (Phase 3 of the unified-transfer refactor). The reader and writer are obtained through `TransferEndpoint::open_reader` / `open_writer`, so the same parallel reader/writer pipeline serves `Local → Local`, `Local → Ssh`, `Ssh → Local`, and `Ssh → Ssh` (when both endpoints are the same client, the engine's planned fast path in Phase 4). Direct I/O is preserved on Local+Local and silently skipped on SSH (SFTP has no equivalent). Four new tests cover basic copy, hash verification, bandwidth throttling, and cancellation.
- `TransferJob` and `TransferWorker` are now endpoint-aware (Phase 4 of the unified-transfer refactor). Every I/O call in the worker — scan, copy, delete, atomic move, symlink recreation, free-space, recycle bin — routes through `TransferEndpoint`. Same-endpoint `Move` is now an O(N) atomic rename instead of N copies + N deletes. New `TransferJob::with_endpoints` constructor for the upcoming call-site migration in Phase 5+. Six new worker tests cover copy, delete, atomic move, symlink preservation, and the circular-symlink cycle-detection guard.
- Copy and Move action handlers migrated to the unified engine (Phase 5). The active/passive panel's `ssh_conn` is mapped to a `TransferEndpoint` per side, so the same `submit_copy_job` / `submit_move_job` path now serves `local ↔ local`, `local ↔ SSH`, `SSH → SSH` (same client, atomic) and `SSH → SSH` (different clients, copy+delete). The legacy `spawn_copy_move_task` modal and the `fs::ops_worker::copy_move` / `helper` modules are gone; `BackgroundOpContext` is now a single-variant enum (`Delete` only) until Phase 6 finishes the migration.
- Delete action handler migrated to the unified engine (Phase 6). Both the no-confirm `handle` path and the `ConfirmDelete` popup now go through the engine with the active panel's endpoint. The legacy `spawn_ssh_delete_task` modal and `fs::ops_worker::delete` are gone. `SharedSshClient::delete_recursive` and `BackgroundOpContext::Delete` are deleted. `BackgroundOpContext` is now an empty enum and will be removed entirely in Phase 10.
- Rename action routed through the unified engine (Phase 7). New `TransferOperation::Rename` variant on the engine: single source, single destination, both endpoints equal to the active panel (cross-panel rename is rejected — use Move for that). `app/actions/fs_ops/rename.rs::commit` now enqueues a `TransferJob::Rename` instead of calling `std::fs::rename` directly. The legacy `AdminOpKind::Rename` retry-as-admin path is gone (the engine has its own retry, and the engine can be extended later to handle elevation if needed).
- CreateLink action routed through the unified engine (Phase 8). New `TransferOperation::CreateLink { kind }` and a new `fs::transfer::job::LinkKind` enum drive the link creation. The create-link popup now enqueues a `TransferJob::CreateLink` instead of calling `fs::create_symlink` / `fs::create_hardlink` directly. The `fs::link` module is gone (its functionality lives in `TransferEndpoint::create_symlink` / `create_hardlink`). Hard links over SSH are rejected with a clear error (SFTP v3 has no `link` command).
- `lefthook` pre-commit hook that runs `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings` before every `git commit`, so formatting and lint regressions get caught at commit time instead of failing the Rustfmt / Clippy CI jobs. Install once per clone with `cargo install lefthook --locked && lefthook install`.
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
- Pairee no longer touches the per-directory `descript.ion` file as a side effect of file operations. Deleting, moving, sending to the recycle bin, or securely wiping a file used to trigger a cleanup call against `descript.ion`; that cleanup is gone. The descriptions file is now only modified by the explicit `DescribeFile` prompt (`Ctrl+Z`) and read by the `Ctrl+6` view.

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

#### Second-pass bug sweep (2026-07-26)

A follow-up audit of the downloader, git, transfer pipeline,
queue, plugin sandbox, external tools, and editor code paths
uncovered 12 more defects. All of them are fixed in this
release.

**Downloader (`src/update/downloader.rs`)**

- Release assets are now downloaded with a hard cap
  (`MAX_ASSET_BYTES = 200 MiB`); a malicious or compromised
  release server returning a 100GB body can no longer fill
  the user's disk.
- The download URL is now required to be `https://`; plain
  HTTP downloads fail closed so a MITM cannot replace the
  binary before the SHA-256 check.
- `validate_filename` rejects empty names, absolute paths,
  `..` traversal segments, NUL bytes, and path separators
  before any byte is written, closing a path-traversal
  vulnerability in which a release asset named
  `../../../etc/cron.d/pairee` would have been materialised
  outside the destination directory.
- Progress is now computed in `f64` so the UI does not get
  stuck at 99.99% on multi-gigabyte downloads.

**External tools (`src/fs/external_tools.rs`)**

- The 7z download now uses a 60-second timeout, a 50 MiB
  size cap, and a *per-PID* temp filename (the previous
  fixed name `pairee_7z_extra.7z` raced with any other
  Pairee instance). A `TempGuard` RAII type removes the temp
  file on every exit path — success, error, panic.

**Git (`src/git/remote.rs`, `src/git/repo.rs`)**

- SSH key authentication now walks the standard key list
  (`id_ed25519`, `id_ecdsa`, `id_rsa`, `id_dsa`) when the
  SSH agent is unavailable. The previous code only tried
  `id_rsa`, which made every fresh `git fetch` from a
  modern ed25519-only setup fail.
- `git pull` no longer uses `CheckoutBuilder::force()` —
  uncommitted local changes now survive a fast-forward
  pull, with a clear conflict surfaced to the user instead
  of silent destruction.
- `clone_repo` now handles HTTPS remotes via
  `git2::Cred::default()` (which honours the user's
  `credential.helper` config). The previous code only
  configured SSH callbacks, so cloning a `https://...`
  URL always failed with a useless "Authentication failed"
  error.

**Editor (`src/app/sys_helpers/editor.rs`)**

- `find_next_in_editor` is now fully char-index safe. The
  pre-fix version did `&line[current_x..]` directly, which
  panicked with `"byte index N is not a char boundary"` the
  moment a user pressed F3 on a line containing accented
  characters, CJK, or emoji. A new `char_slice` helper
  operates entirely in char-index space and converts back
  to byte offsets only for the final `find`/`rfind` call.

**Transfer pipeline (`src/fs/transfer/pipeline.rs`)**

- The reader's "Writer thread disconnected" error has been
  replaced with two distinct messages: `"Transfer cancelled
  by user"` when the user actually cancelled, and a clear
  I/O error message otherwise. The old text made every
  cancellation look like a bug.

**Transfer queue (`src/fs/transfer/queue.rs`)**

- `dequeue` now mutates the job in place instead of
  clone-then-clone-again. `TransferJob` carries options,
  paths and metadata, so the double clone was a measurable
  per-dispatch cost.

**Background tasks (`src/app/app/background.rs`)**

- `process_background_updates` now wraps the
  `take`/restore of `state.progress_rx` in a tiny RAII
  guard. If anything in the middle of the function
  panics, the receiver is still put back and the
  background task can keep delivering progress updates
  instead of seeing its channel close and the progress
  bar freeze forever.

**Plugin sandbox (`src/plugin/sandbox.rs`)**

- `looks_like_version_suffix` is now strict: the suffix
  must contain at least one digit and may only contain
  digits, dots, and dashes. The old
  `chars().all(|c| !c.is_ascii_alphabetic())` accepted
  non-version suffixes like `python3.`, `bash.`,
  `python3;`, and `bash-`, which were potential bypasses
  of the `is_command_safe` command blacklist.

**Bookmarks (`src/app/sys_helpers/bookmarks.rs`)**

- `get_hotlist_bookmarks` now calls
  `directories::UserDirs::new()` once and reads every
  field (`home_dir`, `desktop_dir`, `document_dir`,
  `download_dir`) from the same instance. The previous
  code constructed a fresh `UserDirs` for every field,
  which is four separate env-var-and-passwd-DB scans at
  startup.

**Tests**

- New tests in `app/sys_helpers::editor`: multi-byte
  UTF-8 search must not panic on accented / CJK / emoji
  lines.
- New tests in `update::downloader`: `validate_filename`
  accepts normal names, rejects traversal / absolute /
  control-character names.
- New tests in `plugin::sandbox::is_command_safe`: the
  strict `looks_like_version_suffix` now rejects `.`,
  `;`, `-`, `..` and suffixes with no digit.

#### Third-pass bug sweep (2026-07-26)

A third audit pass focused on the archive pipeline, the
plugin `fs` and `process` bindings, and the transfer
conflict-resolver. 13 defects were found and fixed.

**Archive extraction (`src/fs/archive.rs`)**

- `sevenz-rust 0.6.1` does **not** validate entry names by
  default. An entry named `../../../etc/cron.d/pairee` in
  a 7z archive would be written to
  `dest/../../../etc/cron.d/pairee`, which the OS resolves
  to `/etc/cron.d/pairee`. We now pass a custom extract
  callback to `decompress_file_with_extract_fn` that calls
  `validate_archive_entry_name` on every entry, rejecting
  empty names, `..`, absolute paths, NUL bytes, control
  characters, Windows drive prefixes, and backslashes. The
  zip and tar extract paths re-validate as defence in
  depth, even though their crates check internally.
- The 7z extract callback also refuses to write through a
  pre-existing symlink in the destination tree. A symlink
  planted in `dest_dir` before the extract runs would
  otherwise have been followed by the writer.
- The external 7z CLI (used for RAR/ISO) previously
  concatenated `-o` with the destination directory. If
  the directory happened to start with `-` (a perfectly
  legal path on Linux), 7z would interpret it as an
  unknown option. We now use `std::path::absolute` to
  resolve the destination, then prefix with `./` when
  the result is relative, so `-o<path>` never starts
  with a dash.
- `compress_zip` silently dropped every file inside a
  directory argument; it only added the directory entry
  itself, which is a user-visible data-loss bug. The
  function now pre-walks the tree with
  `collect_files_recursive` (symlink-aware) and then
  `zip.start_file` for every regular file, with a
  correct `total_files` count for the progress bar.
- `compress_zip` used `file_name()` to derive the entry
  name, which panicked on empty source paths. Sources
  that are neither files nor directories now emit a
  clear warning and are skipped.

**Plugin `fs` bindings (`src/plugin/runtime/bindings/fs.rs`)**

- `validate_path_with` previously returned the uncanonicalised
  path verbatim when `canonicalize` failed (the "non-strict"
  branch used by `mkdir` and `rename`). A non-existent path
  outside the workspace therefore bypassed the sandbox check
  entirely. The non-strict path now canonicalises the
  *parent* of the missing target and refuses the operation
  if the parent is outside the sandbox.
- `is_in_sandbox` previously called `std::env::current_dir()`
  on every check, so the sandbox anchor moved if a plugin
  called `Command:cwd` between two calls. The roots are
  now frozen in a `OnceLock` at first use (cwd, config
  dir, cache dir).
- `is_in_sandbox` used to silently fall back to the
  uncanonicalised root on `canonicalize` failure, so any
  path under `/nonexistent-root/` would pass the check.
  The fallback is gone: the roots are canonicalised once
  at startup, and the lookup is straightforward.
- A new `resolve_safe_target` helper canonicalises the
  parent of the write/rename/copy destination and joins
  the original leaf, eliminating the TOCTOU window where
  a local attacker could swap a symlink in the parent
  between canonicalize and the I/O call. `mkdir`, `copy`,
  and `rename` all use it; `rename` and `copy` also
  refuse to operate *over* a pre-existing symlink at the
  destination (which would otherwise let the rename
  follow the symlink to overwrite files outside the
  sandbox).
- `fs.remove("dir_all", …)` and `fs.remove("dir", …)`
  now refuse to follow a symlink target. The previous
  behaviour would happily `remove_dir_all` through a
  symlink and delete the symlink's target. `fs.remove(
  "dir_clean", …)` also refuses any child symlink in
  Secure Mode, for the same reason. `fs.remove(
  "file", symlink)` is still allowed — it unlinks the
  symlink itself, which is the intended use.

**Plugin `process` bindings (`src/plugin/runtime/bindings/process/command.rs`)**

- `Command:env(key, value)` had no Secure-Mode filter
  at all. A plugin could set `LD_PRELOAD=/tmp/evil.so`
  and the child would load the preloaded library on
  startup, running arbitrary code inside the sandbox
  boundary. We now deny a case-insensitive list of
  dynamic-linker hooks: `LD_PRELOAD`, `LD_LIBRARY_PATH`,
  `LD_AUDIT`, `LD_DEBUG`, `LD_DEBUG_OUTPUT`, `LD_BIND_NOW`,
  `LD_PROFILE`, `LD_SHOW_AUXV`, `LD_HWCAP_MASK`,
  `LD_ORIGIN_PATH`, `LD_DYNAMIC_WEAK`, `LD_USE_LOAD_BIAS`
  (Linux/BSD), and the `DYLD_*` family on macOS.
- `Command:cwd(path)` previously had no sandbox check.
  A plugin could `cwd("/etc")` and then `spawn("ls")`,
  which would let the child read arbitrary directories
  even with the binary blacklist in effect. We now
  validate the cwd against the same frozen sandbox
  roots used by the file bindings.
- `:memory(N)` clamped `rlim_cur` and `rlim_max` to `N`
  with no lower bound. A plugin could request
  `memory(1)`, which would cause the child to fail with
  `ENOMEM` before the dynamic linker even loaded —
  a self-DoS. We now clamp the request to a 4 MiB
  floor (`MIN_RLIMIT_AS`) with a warning when the floor
  kicks in, and we documented the per-platform
  `RLIMIT_AS` behaviour (Linux is strict, macOS
  looser, *BSD similar to Linux).

**Transfer conflict resolver (`src/fs/transfer/conflict.rs`)**

- `resolve_filename_conflict` used `rfind('.')` to find
  the last dot, then sliced `&file_name[..dot_idx]`. If
  the name contained a multi-byte UTF-8 character
  immediately before the dot, the slice landed on a
  non-char boundary and the subsequent `format!` call
  panicked with "byte index … is not a char boundary",
  aborting the transfer worker. The fix uses
  `char_indices` to convert the byte offset to a char
  index, then collects via `chars().take(char_idx)` so
  multi-byte characters are respected.
- The conflict loop had no upper bound; a pathological
  case (a million `archivo (1).txt`, `archivo (2).txt`,
  …) would have looped until `counter` overflowed. The
  loop now caps at 10 000 attempts and returns the last
  existing candidate so the caller can surface a "too
  many conflicts" error instead of hanging the worker.

**Tests**

- `fs::archive::tests::test_validate_archive_entry_name_*`:
  the zip-slip / 7z-slip denylist accepts safe names and
  rejects every attack shape.
- `fs::transfer::conflict::tests::test_conflict_resolution_*`:
  multi-byte UTF-8 in base or extension, hidden files,
  no-extension files, and the 10 000-attempt cap.
- `plugin::runtime::bindings::fs::tests::test_fs_*`:
  `copy` over a symlink is refused, `mkdir` works on
  a missing path inside the workspace, `remove_dir_all`
  refuses a symlink, `remove_file` allows a symlink.
- `plugin::runtime::bindings::process::command::tests`:
  `LD_PRELOAD` and friends are blocked in Secure Mode;
  ordinary env vars (`LANG`, `PATH`, `HOME`) are still
  allowed; `cwd_in_sandbox` accepts workspace paths and
  refuses paths outside the workspace; the rlimit
  clamp lifts sub-floor requests to the 4 MiB minimum.

Verification
- `cargo check --all-targets`: clean
- `cargo clippy --all-targets`: 0 warnings
- `cargo test -- --test-threads=1`: 259 passed, 0 failed
