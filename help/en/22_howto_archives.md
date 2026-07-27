# 🔧 How-To: Archives (Compress, Extract, Inspect)

> **Quadrant: HOW-TO** — *problem-oriented.*

Pairee supports the most common archive formats out of the box:

- **Compression**: `zip`, `tar`, `tar.gz`, `tar.bz2`, `tar.xz`, `7z`.
- **Extraction**: `zip`, `tar`, `tar.gz`, `tar.bz2`, `tar.xz`, `7z`,
  `rar` (read-only listing), `gz`, `bz2`, `xz`.

The engine is async: even multi-gigabyte archives are created or
extracted in background workers.

---

## Compress files into an archive (Shift+F1)

**Goal:** bundle the highlighted or tagged files into a new archive.

1. Tag the files (or leave one highlighted).
2. Press `Shift+F1` (or `Files → Add to archive`).
3. In the dialog:
   - **Archive name** — type the output filename including the
     extension. Pairee picks the format from the extension
     (`.zip` → ZIP, `.7z` → 7z, `.tar.gz` → tar+gzip, etc.).
   - **Format** — confirm or override (ZIP, TAR, TAR.GZ, TAR.BZ2,
     TAR.XZ, 7Z).
   - **Compression level** — Normal, Max, Fast, Store.
   - **Include subdirectories** — toggle recursive packing.
   - **Password** — optional, ZIP and 7Z only.
4. Press `Enter`. The progress popup shows the running compression.

> Format detection by extension is automatic. If you type `foo` with no
> extension, Pairee defaults to ZIP.

---

## Extract an archive (Shift+F2)

**Goal:** unpack an archive into the passive panel (or a chosen target).

1. Highlight the archive in the active panel.
2. Press `Shift+F2` (or `Files → Extract files`).
3. In the dialog:
   - **Extract to** — defaults to the passive panel's path; edit if
     you want a subfolder.
   - **Overwrite policy** — Always, Ask, Skip existing.
   - **Preserve paths** — keep the archive's directory structure.
4. Press `Enter`. The extraction runs in the background.

For `.tar.gz` / `.tar.bz2` / `.tar.xz` / `.zip` / `.7z` archives,
Pairee picks the right decoder automatically.

---

## Archive commands (Shift+F3)

**Goal:** inspect or modify an archive **without unpacking the whole
thing**.

1. Highlight an archive.
2. Press `Shift+F3` (or `Files → Archive commands`).
3. A popup shows the available operations. The exact menu depends on
   the archive type:

   | # | Option | Available for |
   | --- | --- | --- |
   | 1 | List contents | all |
   | 2 | Test integrity | zip, 7z, tar.* |
   | 3 | Extract here | all |
   | 4 | Extract to other panel | all |
   | 5 | Add files | zip, 7z, tar.* |
   | 6 | Delete files | zip, 7z, tar.* |

4. Pick the operation and follow the prompts.

> RAR archives are read-only: you can list and extract them, but not
> add or delete entries.

---

## Browse an archive like a folder (Quick View)

**Goal:** peek inside an archive without unpacking it.

1. Highlight the archive in either panel.
2. Press `Ctrl+Q` to toggle Quick View on the passive panel.
3. The passive panel now shows the archive's root entries. Press
   `Enter` to dive into subfolders (Pairee uses a virtual path
   `archive.zip/internal/path`).
4. Press `Ctrl+Q` again to close.

This is also how the **internal viewer** (F3) handles archives: it
shows a listing of the contents.

---

## Single-file decompression

**Goal:** unpack a single `.gz` / `.bz2` / `.xz` file (not a tar
wrapper).

1. Highlight the file.
2. Press `Shift+F2`.
3. Pairee detects the single-file compression and prompts for the
   output name (default: file without the compression extension).
4. Confirm. The output is written in the background.

---

## Common pitfalls

- **Password-protected archives**: Pairee prompts for the password at
  extract time. If you set a password during compression, the
  recipient must enter the same string.
- **Huge archives**: extraction can take time. The progress popup
  shows you the running speed; you can switch screens while it runs.
- **Symlinks inside tar archives**: tar archives containing
  symlinks are extracted safely (links are recreated, not followed
  blindly).

---

## Where to go next

- Background workers: [`50_explanation_architecture`](50_explanation_architecture.md)
- Full keymap: [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
