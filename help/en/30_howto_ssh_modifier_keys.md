# 🔧 How-To: Ctrl / Alt Modifier Keys over SSH

> **Quadrant: HOW-TO** — *problem-oriented.*

This page explains a quirk of plain terminal sessions: when you connect
to Pairee over SSH, **the terminal only sends a keystroke when the
combination is complete** (e.g. `Ctrl+F3` is sent as one event). It
does not send *intermediate* events when you press or release `Ctrl` or
`Alt` alone. As a result, the F-key bar at the bottom of Pairee cannot
automatically switch to the "Ctrl" or "Alt" view when you hold those
keys.

Pairee ships with two solutions: **manual cycling** and **X11
forwarding**.

---

## Solution 1: Manual cycling (`Ctrl+P`)

No third-party software needed. Press **`Ctrl+P`** (or `Ctrl+p`) to
cycle the F-key bar through three states:

| Press | Bar shows |
| --- | --- |
| 1st | **Ctrl** row: F1 Left, F2 Right, F3 Name, F4 Extens, F5 Time, F6 Size, … |
| 2nd | **Alt** row: F1 Left drive, F2 Right drive, F3 View alt, F4 Edit alt, F5 Print, F6 Make link, F7 Find, F8 History, F9 Video, F10 Tree, F11 View hist, F12 Folders hist |
| 3rd | **Default** row: F1 Help, F2 User, F3 View, F4 Edit, F5 Copy, … |

The bar is purely a **visual** hint. The actual bindings work
regardless of the displayed row — `Ctrl+F3` will always sort by name,
`Alt+F1` will always open the left drive menu.

> This works in **every** terminal, including plain SSH without X11.

---

## Solution 2: X11 forwarding (live tracking)

If you want the bar to **update in real time** as you hold `Ctrl` or
`Alt`, enable **X11 forwarding** on your SSH connection. Pairee will
query your local X server to read the physical modifier-key state.

> This is **opt-in**. It works in addition to `Ctrl+P`, never instead.

### Windows host

#### MobaXterm (easiest)

MobaXterm includes an integrated X server. Just create a new SSH
session — X11 forwarding is configured automatically.

#### Windows Terminal / PowerShell / CMD with VcXsrv

1. Download and install **VcXsrv** (or **Xming**).
2. Launch **XLaunch** with:
   - Multiple windows
   - Display number: `0`
   - **Disable access control** ← required to allow remote connections.
3. Connect with the built-in OpenSSH client:

   ```cmd
   ssh -Y user@hostname -p port
   ```

#### PuTTY

1. Open the session settings.
2. **Connection → SSH → X11**.
3. Check **Enable X11 forwarding**.
4. Set **X display location** to `localhost:0`.
5. Make sure VcXsrv (or Xming) is running in the background before
   connecting.

### macOS host

1. Download and install **XQuartz**.
2. Open XQuartz → **Preferences → Security** → check
   **Allow connections from clients**.
3. Connect with X11 forwarding:

   ```bash
   ssh -Y user@hostname -p port
   ```

### Linux host

Linux has X11 built in:

```bash
ssh -Y user@hostname -p port
```

---

## Verifying it works

Inside Pairee, hold `Ctrl`. The F-key bar should switch to the
**Ctrl** row within one refresh. If it does not:

- Confirm your X server is running and your `DISPLAY` is set
  (`echo $DISPLAY` should print something like `:0`).
- Confirm the SSH client forwarded X11 (`echo $DISPLAY` on the remote
  host should not be empty).
- Some Windows terminal emulators strip the X11 socket; try MobaXterm
  if you hit this.

---

## When all else fails

Use **`Ctrl+P`** to lock the bar in the row you want. It survives until
you cycle again or restart Pairee.

---

## Where to go next

- F-key bar reference: [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
- How the F-key bar is rendered: [`50_explanation_architecture`](50_explanation_architecture.md)
