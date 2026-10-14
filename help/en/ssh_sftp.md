# Remote Connections Manual (SFTP, FTP, FTPS, WebDAV, S3)

Pairee can show a remote server in either panel and copy between any two places: local ↔ remote and remote ↔ remote (for example from an FTP server straight to an S3 bucket). The protocols are SFTP (SSH), FTP, FTPS (FTP with explicit TLS), WebDAV (HTTP or HTTPS) and S3 (Amazon S3 and compatible stores such as MinIO, Garage, Ceph or Cloudflare R2).

---

## 1. Opening the Connection Manager

1. **Shortcut Key:** Press **`Ctrl+Shift+S`**.
2. **Dropdown Menu:** **`Files`** (or **`Commands`**) → **`Connect to server...`**.

The connection opens in the panel that was active.

---

## 2. The Connection Dialog

The left column lists the saved connections with their protocol, e.g. `backup (SFTP)`. Use the arrow keys (or click) to show one in the fields on the right. `Tab` / `Shift+Tab` walk through the list, the fields and the buttons.

| Field | Meaning |
| :--- | :--- |
| **Name** | A label for the saved connection. |
| **Protocol** | SFTP, FTP, FTPS, WebDAV, WebDAV (HTTPS) or S3. Change it with `←` / `→` or `Space`. A port that was the old protocol's default follows to the new one's. |
| **Host** (**Bucket** for S3) | The server name or IP address; for S3, the bucket. |
| **Port** | Empty uses the protocol's default (22, 21, 80 or 443). Not used by S3. |
| **User** (**Access key** for S3) | The login name. FTP and WebDAV log in anonymously without one; S3 then uses `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY`. |
| **Password** (**Secret key** for S3) | Stored in the system keyring when saved (see §5). It may stay empty: Pairee asks for it when connecting. For SFTP it is also the passphrase of the key file. |
| **Key Path** | SFTP only: your private key file. Empty: ssh-agent and the default keys in `~/.ssh` are tried. |
| **Folder** | The folder opened after connecting (empty: the server's root). For WebDAV and S3 it is the root of the panel (an S3 key prefix). |
| **S3 options** | S3 only: `endpoint=https://minio.local:9000&region=eu-west-1&path_style=true`. Without an endpoint, AWS is used. |
| **TLS options** | FTPS and WebDAV (HTTPS): `ca=/path/to/cert.pem` trusts the certificates of that PEM file (a private CA, or the server's own self-signed certificate) besides the system's; `insecure=true` turns certificate checks off (Pairee warns every time it connects: anyone on the network path could read the password). Join several with `&`. |

### Certificates (FTPS, WebDAV over HTTPS)
Pairee checks the server's certificate against the system's trusted authorities, like a browser. When a server presents a certificate the system does not trust (self-signed, or from a private authority), Pairee shows its **SHA-256 fingerprint** and asks:
* **Enter** trusts it for that server (host and port) from now on: it is saved as `certs/<host>_<port>.pem` in the configuration folder (delete the file to forget it). Compare the fingerprint with the one the server's administrator gave you first.
* **Esc** refuses it and the connection fails.

For a private authority used by several servers, put its certificate in a PEM file and add `ca=<file>` to the TLS options instead.

Buttons: **Connect**, **Save** (stores the fields under their name, replacing a connection with the same name), **Delete** and **Cancel**. Connecting does not save.

---

## 3. Passwords and Passphrases Asked When Needed

A connection that needs a secret Pairee does not have opens a small **Authentication** prompt while the connection runs in the background:

* an SFTP server that accepts passwords when no key, agent or password worked;
* the passphrase of an encrypted SSH key (the configured key file or a default key in `~/.ssh`);
* the password of an FTP, WebDAV or S3 connection that has a user but no stored password, or whose password was rejected;
* Git fetch, pull and push with an encrypted key in `~/.ssh`.

What you type is masked. `Enter` sends it, `Esc` cancels (the connection then stops with a message). Secrets typed there are used for that login only; save them in the connection to keep them.

---

## 4. Working on a Remote Panel

The panel title shows the protocol and the server, e.g. `[FTP: user@host]`.

* **Navigation:** `Enter` opens folders, `Backspace` / `..` goes up.
* **F5 / F6 (Copy / Move):** between a remote panel and a local one, or between two remote panels of any protocol. Transfers run in the background with the transfer panel's progress, conflict questions, retries, verification and resume. A move within one connection is a server-side rename (and can be undone).
* **F7 (Rename), Make folder, F8 (Delete):** on the server (delete is not undoable).
* **F3 / F4:** view, or edit through a temporary local copy that is uploaded when saved (the server copy is checked for changes first).
* **Ctrl+A (Attributes):** SFTP only.
* Auto-refresh polls remote folders only with `auto_refresh_ssh` on.

---

## 5. Where Secrets Are Kept

Passwords and S3 secret keys are stored in the system keyring (Windows Credential Manager, macOS Keychain, Secret Service on Linux), never in `config.toml`. If the keyring cannot be used they are kept for the session only, unless `passwords_in_config = true` is set under `[security]` in `config.toml`. If the keyring cannot be read at startup (locked, temporarily unavailable), the saved password is left untouched and read again when you connect.

Connections saved by older releases (`[[ssh_presets]]`) are read as SFTP connections and their keyring entries are moved automatically.

---

## 6. Disconnecting

**`Files`** (or **`Commands`**) → **`Disconnect`** returns the panel to the local disk, as does choosing a local drive in the drive menu (`Alt+F1` / `Alt+F2`).
