## [Unreleased]

### Added

### Improved

### Changed

### Deprecated

### Removed

### Fixed

- SFTP: a connection that fails (an unknown host, a wrong password) shows its error over the connection dialog instead of closing it, so the wrong field can be fixed and tried again; a missing field does the same.
- SFTP: the command line of a panel showing a server runs the command on that server, in the panel's folder, with its output shown as it arrives (it used to run on the local machine, in a folder that does not exist there, and fail without a word). Panels that have no shell (FTP, WebDAV, S3, inside an archive) say so. A local command that cannot start or exits with an error now says so too.
- Linux installer: making `sudo pairee` work is now the default answer (`[Y/n]`); it used to be "no", which left `sudo pairee`, and `pairee` in a root shell opened with `sudo su`, as "command not found". When it is not set up, the summary says how to add it later (`--sudo-command`).
