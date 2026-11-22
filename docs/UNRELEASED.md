## [Unreleased]

### Added

- Viewer (F3): text can be selected and copied without opening the editor. Text mode has a cursor line: the arrows, PageUp / PageDown and Home / End move it (the view follows) and a click puts it on a line. Shift with those keys selects whole lines from the cursor, Shift+click selects up to the clicked line, Ctrl+A selects everything and dragging the mouse selects the lines it crosses (scrolling past the edges). Ctrl+C (or Ctrl+Insert) copies the selection, or the cursor line when nothing is selected, up to 32 MB. Esc drops the selection first. The copy also goes to the editor's own clipboard, so it pastes inside Pairee where there is no system clipboard.
- SFTP: the command line of a panel showing a server runs the command on a terminal of that server, so interactive programs work (`vim`, `top`, `sudo` asking for its password), and `command &` opens a terminal screen fed by the server that can be typed into and stopped like a local one. Each command uses a session of its own, opened with the saved login (a password or key passphrase typed at the prompt is kept for it, which also lets a dropped connection reopen without asking).

### Improved

### Changed

### Deprecated

### Removed

### Fixed

- SFTP: a connection that fails (an unknown host, a wrong password) shows its error over the connection dialog instead of closing it, so the wrong field can be fixed and tried again; a missing field does the same.
- SFTP: the command line of a panel showing a server runs the command on that server, in the panel's folder (it used to run on the local machine, in a folder that does not exist there, and fail without a word). Panels that have no shell (FTP, WebDAV, S3, inside an archive) say so. A local command that cannot start or exits with an error now says so too.
- Windows: paths on a server read with `/` throughout (`/home/deck/umbral`, not `/home\deck\umbral`) in the panel title, the command line prompt, the info dialog, the viewer, the editor, file compare and the transfer dialogs.
- Linux installer: making `sudo pairee` work is now the default answer (`[Y/n]`); it used to be "no", which left `sudo pairee`, and `pairee` in a root shell opened with `sudo su`, as "command not found". When it is not set up, the summary says how to add it later (`--sudo-command`).
