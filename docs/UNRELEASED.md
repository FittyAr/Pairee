## [Unreleased]

### Added

- Viewer (F3): text can be selected and copied without opening the editor. Text mode has a cursor line: the arrows, PageUp / PageDown and Home / End move it (the view follows) and a click puts it on a line. Shift with those keys selects whole lines from the cursor, Shift+click selects up to the clicked line, Ctrl+A selects everything and dragging the mouse selects the lines it crosses (scrolling past the edges). Ctrl+C (or Ctrl+Insert) copies the selection, or the cursor line when nothing is selected, up to 32 MB. Esc drops the selection first. The copy also goes to the editor's own clipboard, so it pastes inside Pairee where there is no system clipboard.
- SFTP: the command line of a panel showing a server runs the command on a terminal of that server, so interactive programs work (`vim`, `top`, `sudo` asking for its password), and `command &` opens a terminal screen fed by the server that can be typed into and stopped like a local one. Each command uses a session of its own, opened with the saved login (a password or key passphrase typed at the prompt is kept for it, which also lets a dropped connection reopen without asking).

### Improved

- Command line: commands run on a terminal screen inside Pairee's layout instead of taking over the whole terminal until they end. The screen is an emulated terminal (colors, cursor, full-screen programs, scrollback), locally and on SSH panels, so a `sudo su` or `vim` no longer hides the menu and the key bars. `Alt+O` brings the panels back while the program keeps running (`Ctrl+Tab` or the Screens list return to it); the bottom border says which keys do what; once it ended, `Enter` or `Esc` closes it and the panels are read again. A trailing `&` still works and does the same. Cursor position queries are answered, so programs that wait for one (PowerShell under ConPTY) draw at once.

### Changed

- Building Pairee from source needs Rust 1.99 or newer (was 1.96): file times are now set with the standard library (`std::fs::set_times`), which replaces the deprecated `filetime` crate. Dependencies updated to their latest versions (rusty-s3 0.12 among them), which also clears the new security advisories of libz-sys, url and comrak.

### Deprecated

### Removed

### Fixed

- Connection dialog: Enter on a saved connection in the list connects to it (it only loaded its fields, and Connect had to be reached by hand). A failed save shows its error over the dialog instead of closing it.
- Screens list (F12): the box is as wide as its key hint and tall enough for it, so the hint is no longer cut and the screens show.

- SFTP: a connection that fails (an unknown host, a wrong password) shows its error over the connection dialog instead of closing it, so the wrong field can be fixed and tried again; a missing field does the same.
- SFTP: the command line of a panel showing a server runs the command on that server, in the panel's folder (it used to run on the local machine, in a folder that does not exist there, and fail without a word). Panels that have no shell (FTP, WebDAV, S3, inside an archive) say so. A local command that cannot start or exits with an error now says so too.
- Windows: paths on a server read with `/` throughout (`/home/deck/umbral`, not `/home\deck\umbral`) in the panel title, the command line prompt, the info dialog, the viewer, the editor, file compare and the transfer dialogs.
- Linux installer: making `sudo pairee` work is now the default answer (`[Y/n]`); it used to be "no", which left `sudo pairee`, and `pairee` in a root shell opened with `sudo su`, as "command not found". When it is not set up, the summary says how to add it later (`--sudo-command`).
