# 📖 Reference: SSH Dialog Fields and SFTP Operations

> **Quadrant: REFERENCE** — *information-oriented.*

Quick reference for the SSH connection dialog and the SFTP operations
available on a connected panel. For the workflow, see
[`23_howto_ssh_sftp`](23_howto_ssh_sftp.md).

---

## 1. Connection dialog fields

| Field | Type | Notes |
| --- | --- | --- |
| **Preset Name** | string | Optional nickname. Required if you want to save a bookmark. |
| **Host** | string | IP or domain. |
| **Port** | integer | Default `22`. |
| **Username** | string | Remote login. |
| **Password** | string | Plain password **or** the passphrase to unlock the private key. |
| **Key Path** | path | Absolute path to a private key (e.g. `~/.ssh/id_ed25519`). Leave blank for password or agent auth. |

The dialog also exposes three buttons:

| Button | Effect |
| --- | --- |
| **Connect** | Open the connection. |
| **Save** | Save the current fields as a bookmark (requires a Preset Name). |
| **Load** | Populate the fields from the highlighted bookmark. |
| **Delete** | Remove the highlighted bookmark. |

> The left column lists every saved bookmark. Use the arrow keys to
> highlight, then **Load** to populate the fields, then **Connect**.

---

## 2. Connection states

| State | Title bar | What works |
| --- | --- | --- |
| Disconnected | `<local path>` | Everything. |
| Connecting | `Connecting to user@host:port…` | Only the cancellation button. |
| Connected | `[SSH: user@host]` | All SFTP operations. |
| Failed | `Connection failed: <reason>` | Nothing. Close the dialog and retry. |

---

## 3. SFTP operations

Once the active panel is in SFTP mode (`[SSH: ...]` title), the
following F-key actions are remapped to the **remote** filesystem:

| Action | Effect on remote |
| --- | --- |
| `Enter` | Open the highlighted folder or run the matching file association. |
| `Backspace` | Go to the parent directory. |
| `F3` | View (downloads a temp buffer, opens in the internal viewer). |
| `F4` | Edit (downloads a temp buffer, opens in the internal editor, uploads on save). |
| `F5` | Copy to the opposite panel (download if opposite is local, upload if opposite is SFTP, server-to-server if both SFTP). |
| `F6` | Move. |
| `F7` | Rename. |
| `F8` | Recursive delete. |
| `Insert` / `Space` | Tag for bulk operations. |
| `Gray+` / `Gray-` / `Gray*` | Bulk select by glob / unselect / invert. |
| `Ctrl+R` | Re-read the remote listing. |
| `Ctrl+\` | Open the Folder Shortcuts dialog (works on the local side, not the remote). |

> The Transfers panel (`Ctrl+T`) lists every running and completed
> transfer. The progress popup shows the same data as for local
> copies: file, speed, ETA, total.

---

## 4. Bookmark file format

Bookmarks are stored as a TOML list under `ssh_bookmarks` in
`settings.toml`:

```toml
[[ssh_bookmarks]]
name = "Production"
host = "ssh.example.com"
port = 22
user = "deploy"
key_path = "/home/me/.ssh/id_ed25519"
# password is intentionally NOT persisted
```

> Pairee **does not** store passwords in the config file. When you
> reload a bookmark, leave the password field empty or fill it in
> again.

---

## 5. Host key handling

- On the **first** connection to a host, Pairee shows the host key
  fingerprint and asks you to trust it. Accepted fingerprints are
  stored in `known_hosts` (alongside your system `~/.ssh/known_hosts`).
- If a host's key **changes**, Pairee refuses the connection and
  asks whether to update. Confirm only if you are sure the server
  was reinstalled.
- Fingerprint algorithms supported: `SHA256:...` (modern OpenSSH) and
  the legacy MD5 colon-separated form, for backward compatibility.

---

## 6. Authentication methods

| Method | When Pairee uses it |
| --- | --- |
| **Password** | You typed a value in the `Password` field and left `Key Path` blank. |
| **Public key (file)** | `Key Path` is set and the file is readable. The `Password` field is treated as the **passphrase** to unlock the key. |
| **SSH agent** | `Password` empty, `Key Path` empty, and a system agent is reachable. |
| **Keyboard-interactive** | Falls back automatically when the server demands it. |

> Public-key formats accepted: `RSA`, `ECDSA`, `Ed25519`. Legacy
> `DSA` is **not** accepted.

---

## 7. Common error messages

| Error | Cause | Fix |
| --- | --- | --- |
| `Connection refused` | Wrong host or port, or the SSH daemon is not running. | Verify host/port. Check `systemctl status sshd` on the server. |
| `Permission denied (publickey)` | Server rejected the key. | Check `~/.ssh/authorized_keys` on the server. Verify file permissions (`chmod 600`). |
| `Host key verification failed` | The host's key changed. | Confirm the server was reinstalled, then accept the new key. |
| `Connection timed out` | Firewall or routing. | Open the port on the server firewall; check your ISP. |
| `No supported auth methods` | Server has no auth method Pairee can use. | Enable `PasswordAuthentication` or `PubkeyAuthentication` in `sshd_config`. |

---

## Where to go next

- SSH workflow: [`23_howto_ssh_sftp`](23_howto_ssh_sftp.md)
- Modifier keys over SSH: [`30_howto_ssh_modifier_keys`](30_howto_ssh_modifier_keys.md)
