## [Unreleased]

### Added

- Git: fetch, pull and push now work with SSH remotes (`git@...`), using ssh-agent or your default keys in `~/.ssh`.

### Improved

### Changed

- Git: pull follows the branch's configured upstream instead of always using the same-named remote branch.
- Git: aborting a merge keeps the local changes you had in other files, like `git merge --abort`.

### Deprecated

### Removed

### Fixed

- Git: pull and fast-forward merges no longer discard uncommitted changes; they stop with an error when your local changes would be overwritten.
- Git: committing after a merge records both parents and ends the merge; committing after a cherry-pick or revert clears its in-progress state.
- Git: a push the remote rejects is reported as an error instead of a success.
- Git: a wrong or passphrase-protected SSH key no longer makes authentication retry forever.
- Git: rebase skips commits that are already applied instead of aborting.
