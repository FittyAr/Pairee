# 🔧 How-To: SSH and SFTP Remote Connections

> **Quadrant: HOW-TO** — *problem-oriented.*

Pairee ships a built-in SSH client and an SFTP backend. Once connected,
the active panel becomes a remote file browser, and you can copy
between local and remote with the same `F5` / `F6` you already use for
local files.

---

## Open the connection dialog

There are three ways to launch the SSH dialog:

| Trigger | Steps |
| --- | --- |
| Hotkey | `Ctrl+Shift+S` |
| Menu | `Left` (or `Right`) → `Connect to SSH…` |
| Drive menu | `Alt+F1` (left) / `Alt+F2` (right) → pick `[Connect SSH]` |

A modal appears with the connection fields on the right and your saved
bookmarks on the left.

---

## Connect for the first time

1. Fill in the connection fields:

   | Field | Example | Notes |
   | --- | --- | --- |
   | Preset Name | `Production API` | Optional nickname, used for bookmarks. |
   | Host | `ssh.example.com` or `192.168.1.50` | |
   | Port | `22` | Default SSH port. |
   | Username | `deploy` | |
   | Password | `••••••` | OR the passphrase to unlock your key. |
   | Key Path | `/home/me/.ssh/id_ed25519` | Leave blank to use password or SSH agent. |

2. Click **`Connect`** (or press `Enter`).

3. On the **first** connection, Pairee will show the host key and ask
   you to trust it. Verify the fingerprint, then accept.

4. When the connection is established, the **active panel** becomes a
   remote browser. The title bar shows
   `[SSH: username@host]`.

---

## Bookmark the connection (preset)

1. After filling the fields, type a unique **Preset Name** (e.g.
   `Work staging`).
2. Click **`Save`**.
3. The preset is stored in your `settings.toml` and shows up in the
   left list next time you open the dialog.

To **load** a saved preset: pick it in the left list, click `Load`,
then `Connect`.

To **delete** a preset: pick it, click `Delete`.

---

## Navigate the remote panel

| Key | Effect |
| --- | --- |
| `Enter` | Open the highlighted folder, or run associations on a file. |
| `Backspace` | Go to the parent directory. |
| `Ctrl+R` | Re-read the remote listing. |
| `F3` | View a remote file (text or hex). |
| `F4` | Edit a remote file in the internal editor (uses a local temp buffer and uploads on save). |
| `F7` | Rename a remote file. |
| `F6` | Move remote files between directories. |
| `F8` | Recursive delete on the remote. |
| `Alt+F1` / `Alt+F2` | Drive menu (use this to switch to a local disk). |

---

## Transfer files local ↔ remote

The two panels act independently: one can be local, the other SFTP
(or both local, or both SFTP to two different hosts).

### Upload (local → remote)

1. Focus the **local** panel; tag the files to upload.
2. Make sure the **remote** panel shows the destination folder.
3. Press `F5` (copy) or `F6` (move).

### Download (remote → local)

1. Focus the **remote** panel; tag the files.
2. Make sure the **local** panel shows the destination folder.
3. Press `F5` (copy) or `F6` (move).

The transfer runs in a background worker. A progress popup shows:

- Current file
- Bytes per second
- Elapsed / ETA
- Total bytes
- An overall progress bar

You can switch screens while the transfer is running.

> The `Copy` button in the SSH dialog **does not transfer files**; it
> copies the dialog fields to the clipboard. Use `F5` / `F6` to move
> actual files.

---

## Disconnect

| Trigger | Steps |
| --- | --- |
| Menu | `Left` (or `Right`) → `Disconnect SSH` |
| Drive menu | `Alt+F1` / `Alt+F2` → pick any local drive (e.g. `/` or `C:`) |

The active panel returns to a local disk view.

---

## Common pitfalls

- **Host key changed**: Pairee refuses the connection and asks whether
  to update. Only do this if you are sure the server was reinstalled.
- **Wrong key permissions**: on Linux/macOS, the SSH client refuses
  keys readable by others. `chmod 600 ~/.ssh/id_*`.
- **No password / agent**: leave `Password` empty **and** `Key Path`
  empty to let the system SSH agent (if any) handle authentication.
- **Time zone / locale**: file timestamps and listings are rendered in
  UTC by default; Pairee follows the local time zone for display.

---

## Where to go next

- Field reference for the dialog: [`44_reference_ssh_fields`](44_reference_ssh_fields.md)
- Modifier keys over SSH: [`30_howto_ssh_modifier_keys`](30_howto_ssh_modifier_keys.md)
- Background transfers: [`50_explanation_architecture`](50_explanation_architecture.md)
