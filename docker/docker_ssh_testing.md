# testing Pairee over SSH in Docker (Bottom Bar Modifiers)

This guide explains how to build, run, and test the **Pairee** file manager over SSH inside a Debian Docker container. Specifically, we will verify the bottom bar's functionality, which changes dynamically depending on the modifier keys (`Ctrl` and `Alt`).

---

## 1. Why SSH and X11 Forwarding are Required
Standard SSH and terminal protocols do not transmit key-release events or the raw states of modifier keys (like holding down `Ctrl` or `Alt`). They only send escape codes when a complete key combination (like `Ctrl+C` or `Alt+A`) is pressed.

To support live updating of the bottom bar, Pairee implements two mechanisms:
1. **Dynamic X11 Polling (Linux)**: When run over an SSH session with **X11 Forwarding** enabled, Pairee queries the host's physical keyboard status using X11's `XQueryKeymap`. This allows the interface to update instantly when you hold down `Ctrl` or `Alt`.
2. **Manual Override (`Ctrl+p`)**: When X11 is not available, you can press `Ctrl+p` (or `Ctrl+P`) to cycle the bottom bar states manually (`None` -> `Control` -> `Alt` -> `None`).

---

## 2. Start the Docker Container

Make sure Docker is running on your host system, then execute these commands from the project root:

```powershell
# Navigate to the docker configuration directory
cd docker

# Build and start the container in the background
docker compose up --build -d
```

This starts the SSH daemon on host port **`2222`** and mounts your project root to `/workspace` inside the container.
* Credentials:
  - **Root user**: `root` / password: `root`
  - **Normal user**: `pairee` / password: `pairee` (with sudo privileges)

---

## 3. SSH Client Setup (with X11 Forwarding)

To test the live modifier key updates, you must configure your SSH client with X11 forwarding.

### Option A: MobaXterm (Easiest & Recommended)
MobaXterm includes a built-in X server and enables X11 forwarding automatically.
1. Click **Session** -> **SSH**.
2. Set **Remote Host** to `127.0.0.1` and **Port** to `2222`.
3. Set **Username** to `root` or `pairee`.
4. Click **OK** to connect.
5. In the terminal tab, you will see a message: `X11-forwarding : ✔ (enabled)`.

### Option B: Windows Terminal / PowerShell / CMD (Using VcXsrv)
1. Install an X Server on Windows, such as **VcXsrv** (highly recommended) or **Xming**.
2. Run **VcXsrv** via the "XLaunch" wizard:
   - Select **Multiple windows** and click Next.
   - Keep default display number (`0`) and click Next.
   - **Crucial**: Check the option **"Disable access control"** (to allow connections from Docker/WSL), then click Next and Finish.
3. Open Windows Terminal and run:
   ```powershell
   ssh -Y root@127.0.0.1 -p 2222
   ```
   *(Note: The `-Y` flag enables trusted X11 forwarding).*

### Option C: PuTTY
1. Start PuTTY and enter **Host Name**: `127.0.0.1` and **Port**: `2222`.
2. In the left category tree, expand **Connection** -> **SSH** -> **X11**.
3. Check the box **"Enable X11 forwarding"**.
4. Set **X display location** to `localhost:0`.
5. Return to the "Session" category, enter a name in "Saved Sessions", click "Save", and then click "Open".
6. *(Requires an X Server like VcXsrv running on Windows).*

---

## 4. How to Test and Verify

### Step 1: Verify X11 Forwarding is Working
Once logged into the container via SSH:
1. Run `echo $DISPLAY`. It should output a value like `localhost:10.0` or similar.
2. Run the test X11 application:
   ```bash
   xeyes
   ```
   A graphical window with moving eyes should pop up on your Windows desktop. Close the window to continue.

### Step 2: Build and Run Pairee
1. Navigate to the mounted workspace:
   ```bash
   cd /workspace
   ```
2. Build the application for Linux:
   ```bash
   cargo build
   ```
   *(Note: Build outputs are stored in `target-linux/` to prevent interference with your local Windows cargo builds).*
3. Run the application:
   ```bash
   cargo run
   ```

### Step 3: Test Modifier Key Functionality

#### Test Case 1: Physical Modifier Key Polling (X11 active)
1. Focus the Pairee application running inside the SSH session.
2. **Press and hold** the physical `Ctrl` key. 
   - *Result*: The bottom bar's keys must immediately change to show `Ctrl` functions (`F3: Name`, `F4: Extens`, etc.).
   - *Result*: Releasing the `Ctrl` key must immediately return the bottom bar to its default state.
3. **Press and hold** the physical `Alt` key.
   - *Result*: The bottom bar's keys must immediately change to show `Alt` functions (`F3: View`, `F4: Edit`, etc.).
   - *Result*: Releasing the `Alt` key must immediately return the bottom bar to its default state.

#### Test Case 2: Manual Modifier Cycle (X11 disabled / Default SSH)
1. Close your SSH session and reconnect **without** X11 forwarding (e.g., standard `ssh root@127.0.0.1 -p 2222` without `-Y` or X server running).
2. Run `cargo run`.
3. Press `Ctrl+p` (or `Ctrl+P`).
   - *Result*: The bottom bar should toggle to `CONTROL` mode (showing `F3: Name`, etc.) and stay locked there.
4. Press `Ctrl+p` again.
   - *Result*: The bottom bar should toggle to `ALT` mode (showing `F3: View`, etc.) and stay locked there.
5. Press `Ctrl+p` a third time.
   - *Result*: The bottom bar should return to `Default` mode.
