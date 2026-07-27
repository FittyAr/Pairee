# Manual smoke test procedure

This document is the runbook for the manual checks that
cannot be automated in CI. It is meant to be executed by
a developer on a real machine after the security-review
fixes from branch `refactor-plugins` (commits `a48e3c1`
through `bbd2048`).

The test matrix is split into two parts:

1. **Elevation helper (UAC / sudo)** — verifies that the
   C1 (temp-file perms), C2 (policy reset), and M1
   (compress/extract fail into policy) fixes actually
   prompt the user for elevation and complete the
   operation as admin.
2. **Compress / Extract (3 formats × 2 directions)** —
   verifies that the A1–A10 pipeline still works
   end-to-end and the L2 mode preservation fix is
   observable in the produced archive.

If you only have time for one of the two, do the
**elevation helper** section first — that is where the
actual security improvements live, and a missed UAC
prompt means we are silently swallowing failures.

---

## 1. Elevation helper

### 1.1 Verify the temp file is not world-readable (Unix)

The fix for C1 stores the ops JSON in a `tempfile::Builder`
output with mode 0600. Quick check while a copy is in
flight:

```bash
# Terminal 1: launch the copy
./target/debug/pairee
# In the UI, F5 a file from a directory you do not own
# (e.g. /etc/hostname as a normal user).
# Decline the UAC / sudo prompt when it appears.

# Terminal 2 (during the popup):
ls -l /tmp/pairee_op_*.json 2>/dev/null
```

Expected: either no file (the popup is shown before the
helper runs and the file is created) or, if you
trigger the retry by pressing **Yes**, a brief flash of
`-rw------- 1 $USER $USER ...` for the duration of the
helper. **Never** `-rw-r--r--`.

If the perms are wrong, the helper is leaking the
target paths to any local user. **Do not ship.**

### 1.2 Retry-as-admin actually retries (Windows)

1. Launch `./target/release/pairee` as a normal user.
2. Navigate one panel to `C:\Windows\System32\drivers\etc\hosts`.
3. Press **F5** (Copy).
4. In the copy dialog, choose the other panel's
   directory as the destination.
5. Press Enter.

Expected sequence:
1. The file fails with `Access is denied` and a red
   `🔐 Access denied` line appears in the transfer
   log.
2. After the worker finishes, a yellow
   **Permission prompt** popup appears with the list of
   failed files and the line
   `Permission denied (os error 5)` (the `sample_error`
   from L3).
3. Press **Y**. Windows shows a UAC dialog.
4. After you accept UAC, the copy completes and the
   panel refreshes. The log line reads
   `🔐 Elevated helper completed; refreshing panels`.
5. Pressing **N** or **Esc** should leave the file
   uncopied and the panel untouched.

If the popup does not appear, the policy is not
accumulating — debug `PromptPolicy::on_file_error` and
verify that `report_file_failure` is being called from
`emit_file_failed`.

### 1.3 Retry-as-admin actually retries (Unix)

Same as 1.2 but with `/etc/shadow` (or any file the
current user cannot read but root can).

```bash
./target/release/pairee
# F5 the file, decline, then press Y in the popup.
# Expect: terminal shows "Requesting administrator
# privileges...", sudo prompts for your password, the
# copy completes, the panel refreshes.
```

If the perms in `/tmp/pairee_op_*` (during the popup)
are `0644`, see 1.1.

### 1.4 The popup does not appear for SSH

The policy must downgrade SSH `AccessDenied` to
`FileError::IoError` (the helper runs locally and
cannot escalate the remote server). To verify:

1. Connect to an SSH panel.
2. Try to read a file on the remote that your SSH user
   does not have permission for (e.g. `/etc/shadow` on
   the remote).
3. Expected: the file fails with a plain
   `Permission denied (os error 13)` line, **no**
   PermissionPrompt popup at the end of the job. The
   per-file `PermissionDenied` event is also suppressed
   for SSH (we log it as a regular failure instead).

If the popup does appear for an SSH-only failure, the
`is_remote_op` branch in `report_file_failure` is not
firing — that means C2 is still being applied to SSH
operations, which is wasted UX.

---

## 2. Compress / Extract

### 2.1 Local → Local round trip (every format)

For each of ZIP, TAR.GZ, 7Z:

1. In the UI, navigate to a directory with at least
   three files including one with a non-default mode
   (`chmod 600 /tmp/pairee_test/private.txt`).
2. Press the Compress shortcut.
3. Pick the format and a destination archive path.
4. After the compress finishes, press the Extract
   shortcut on the produced archive and pick a fresh
   destination directory.
5. Verify the files are present, the content matches,
   and the `private.txt` in the extracted tree still has
   mode 0600 (this is the L2 fix).

### 2.2 Level 0 in ZIP

In the Compress popup, set level to 0. The resulting
`.zip` should be **at least** the sum of the source
file sizes (no DEFLATE overhead). If it is smaller,
the level=0 path is going through the deflate branch
instead of the `Stored` branch.

### 2.3 7Z level (known limitation)

7Z level is a no-op. The sevenz-rust 0.6.1 crate panics
with "attempt to multiply with overflow" if you try to
set the LZMA level via `set_content_methods`. The
`compress_sevenz` function captures the user's level
and shows it in the UI, but the encoded archive uses
the crate default. A `TODO` in `pipeline.rs` near
`compress_sevenz` documents this. Do not treat a
"level=9 archive same size as level=1" result as a
bug; it is the documented limitation.

### 2.4 Path traversal rejected

Create a tar.gz that contains an entry with a literal
`..` in the path (or, on Windows, `C:/evil.txt`):

```bash
mkdir /tmp/pairee_test/evil
cd /tmp/pairee_test
tar czf ../evil.tar.gz --transform 's,^,../,' .
```

Open the archive in the UI and try to extract. The
extract should fail with one of:
- `archive entry has '..': ../evil/...`
- `archive entry has drive / device prefix: ...`

(see M3). The destination directory must be untouched.

---

## 3. Sanity checklist (run every time)

- [ ] `cargo build --release` produces a binary
- [ ] `cargo test` — 322 passed, 0 failed
- [ ] `cargo clippy --all-targets` — 0 warnings
- [ ] `cargo fmt --all -- --check` — clean

If any of the four fail, the security review is not
truly complete. Stop and fix.
