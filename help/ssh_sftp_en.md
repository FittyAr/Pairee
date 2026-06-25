# SSH & SFTP Remote Connections Manual

Pairee features a fully integrated SSH client and SFTP protocol backend, enabling you to manage files and folders on remote Unix/Linux or Windows SSH servers directly within one of the standard navigation panels.

---

## 1. Establishing a Connection

There are three ways to launch the SSH connection prompt:
1. **Shortcut Key:** Press **`Ctrl+Shift+S`**.
2. **Dropdown Menu:** Select **`Files`** (or **`Commands`**) -> **`Connect SSH...`** from the top menu.
3. **Drive Selection Panel:** Press **`Alt+F1`** (left panel) or **`Alt+F2`** (right panel) to open the disk menu, and choose the item labeled **`[Connect SSH]`**.

---

## 2. The SSH Connection Dialog Fields

The connection prompt contains the following configuration parameters:
* **Preset Name:** A user-friendly label/nickname for this connection (e.g., `Production API Server`). If provided, you can save it as a bookmark.
* **Host:** The remote IP address or domain name (e.g., `192.168.1.50` or `ssh.example.com`).
* **Port:** The remote SSH listening port (defaults to `22`).
* **Username:** The login account name on the remote host.
* **Password:** The password for authentication, OR the passphrase to decrypt your private key file.
* **Key Path:** The absolute local path to your SSH private key file (e.g., `/home/user/.ssh/id_rsa`). Leave blank to use password-only authentication or standard SSH Agent keys.

---

## 3. SSH Bookmarks (Presets)

* **Saving Bookmarks:** Fill in the connection details, type a unique **Preset Name**, and click **`[Save]`**. The preset is stored in your configuration file.
* **Loading Bookmarks:** The left column of the connection dialog lists saved bookmarks. Use the Arrow keys or click to select a preset, press **`[Load]`** to populate the fields, and click **`[Connect]`** (or press `Enter`) to log in.
* **Deleting Bookmarks:** Select a preset in the list and click **`[Delete]`**.

---

## 4. Navigating Remote Filesystems (SFTP)

Once a connection is established, the active panel transitions into SFTP mode:
* The panel title updates dynamically to: `[SSH: username@host]`.
* **Basic Navigation:**
  - **`Enter`**: Open the highlighted folder or run associations on files.
  - **`Backspace`** or **`..`**: Navigate to the parent directory.
* **File Operations:**
  - **`F7`** (MkDir): Create a folder on the remote server.
  - **`F6`** (Rename/Move): Rename or relocate remote files directly on the server.
  - **`F8`** (Delete): Recursively delete files and folders on the remote server.
  - **`F3`** (Viewer): View remote file contents in plain text or hex modes.
  - **`F4`** (Editor): Edit text files directly on the remote server. Pairee handles temp buffers automatically.

---

## 5. Bidirectional File Transfers

You can copy or move files between your local system and the remote server asynchronously:
* **Upload:** Focus on your local file panel, highlight/tag the files to transfer, and press **`F5`** (Copy) or **`F6`** (Move) to transfer them to the active remote folder in the opposite panel.
* **Download:** Focus on the SFTP remote panel, highlight/tag remote files, and press **`F5`** (Copy) or **`F6`** (Move) to download them to the active local folder in the opposite panel.
* **Background Worker:** All transfers run on an asynchronous worker thread. A progress popup displays the current file name, transfer speed (MB/s), elapsed time, total size, and a progress bar. You can switch screens while transfers complete in the background.

---

## 6. Disconnecting

To close the SSH session and restore the panel to your local disk:
1. Open the top menu: **`Files`** (or **`Commands`**) -> **`Disconnect SSH`**.
2. Or open the drive selection panel (`Alt+F1` / `Alt+F2`) and choose any local mount point (e.g. `/` or `C:`).
