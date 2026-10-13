## [Unreleased]

### Added
- Copies resume where they stopped: a copy interrupted by a crash, a power cut or a failed attempt continues from the part already written (after checking it matches the source).
- When a copy overwrites a file, the old file goes to the trash, so undoing the copy brings it back.

### Improved
- Moving files or folders within one drive is instant: they are renamed instead of copied and deleted. Moves to another drive, moves with "Verify after copy" and filtered moves still copy.
- Copies and moves to and from SSH servers now ask (or follow the setting) when a file exists, retry failed files, verify copies, keep times and permissions, and write each file under a temporary name first.
- Direct I/O ("Disable write cache") now bypasses the cache for every whole block of a file, not only for files whose size is a multiple of 4 KiB.

### Changed
- "Verify after copy" reads every copy back from the disk and compares it with the source; a file that does not match is retried and, if it still differs, reported as failed. Files copied without verification are no longer marked as verified in the job log.

### Deprecated

### Removed

### Fixed
- Copying or moving a file onto itself (both panels on the same folder, "Overwrite") emptied or deleted it. It is now refused with an error.
- Overwriting a file was not safe: a cancelled or failed copy left a cut-down file and the original was lost. Copies are now written to `<name>.pairee-part`, flushed to disk and only then put in place; cancelling keeps the original.
- Copying a folder into one of its own subfolders nested copies until the path got too long. It is now refused, and loops of links are detected when links are followed.
- Copying a link changed the permissions and times of the file it pointed to; relative links became absolute (and broke when moved); and replacing a folder with a link could delete the folder. Links are now copied as they are and a folder is never replaced by a link.
- A move reported success even when the source could not be deleted (for example a read-only file on Windows); it is now reported as failed.
- Synchronizing folders with "follow links" could delete files outside the synchronized folders through a link; deletions never follow links now.
- Deleting links to folders and junctions failed on Windows.
- Unreadable folders were silently left out of copies, moves, folder imports into archives and SSH transfers; they are now reported.
- A link to a folder on an SSH server no longer stops a whole copy: it is skipped with a warning.
- A broken link at the destination was followed and the copy written where it pointed; it is now replaced like any other file.
- Progress could pass 100 % when files were retried.
- The low disk space warning checked a destination folder that did not exist yet.
- Alternate data streams were copied after the read-only attribute was set, so they were lost; the "copy streams" option now controls them on its own, and long paths work for attributes, streams and permissions on Windows.
- The transfer speed kept being reported after a job ended.
