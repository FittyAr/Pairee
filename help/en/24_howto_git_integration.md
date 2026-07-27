# 🔧 How-To: Git Integration

> **Quadrant: HOW-TO** — *problem-oriented.*

Pairee includes a full Git dashboard that operates on the repository
containing the active panel's path. It is built on `libgit2` and runs
its operations in background workers, so the UI never blocks on
network or disk.

---

## Open the Git dashboard

| Trigger | Steps |
| --- | --- |
| Hotkey | `Alt+G` (or `Alt+g`) |
| Menu | `Left` (or `Right`) → `Git` (only shown if the active path is inside a repo) |
| Auto-detect | If `git_auto_detect` is enabled in `Configuration → Git`, Pairee scans up the directory tree to find the repo root as you navigate. |

A modal opens with **four tabs**. Use `Tab` / `Shift+Tab` to cycle.

---

## Tab 1: Status

Lists every file in the working tree that differs from `HEAD`, with
a one-letter prefix:

| Prefix | Colour | Meaning |
| --- | --- | --- |
| `M` | Yellow | **Modified** in the working tree. |
| `A` | Green | **Added** to the index. |
| `D` | Red | **Deleted** from the working tree. |
| `?` | Dark grey | **Untracked** (new, not in `.gitignore`). |
| `R` | Cyan | **Renamed**. |
| `!` | Magenta | **Conflicted** (unmerged). |

| Key | Effect |
| --- | --- |
| `Space` | Toggle staging for the highlighted file. |
| `c` | **Commit** all staged changes (opens the commit dialog). |
| `d` | **Diff** the highlighted file against `HEAD`/index. |
| `s` | **Stash** the current changes (prompts for an optional message). |
| `r` | **Refresh** the status list. |
| `Esc` | Close the dashboard. |

### Commit flow

1. Press `Space` on each file you want to include in the commit
   (only the staged files are committed).
2. Press `c`.
3. Type a commit message. Pairee writes the message into the repo's
   `COMMIT_EDITMSG` buffer, so the editor hook (if any) applies.
4. Confirm.

> Empty messages are rejected.

### Diff viewer

Press `d` on any status row. The diff opens in the internal viewer
with red/green highlighting. Press `F4` inside the diff to toggle
text/hex (rarely useful for a diff) or `F7` to search.

---

## Tab 2: Log

Shows the commit history of the current branch, newest first.

| Column | Meaning |
| --- | --- |
| Hash | First 7 hex chars of the commit SHA. |
| Date | `YYYY-MM-DD` in your local time zone. |
| Author | The author name (as configured by `git config user.name`). |
| Message | First line of the commit message. |

| Key | Effect |
| --- | --- |
| `Enter` | **Checkout** the highlighted commit (puts you in **detached HEAD**; you will be asked to confirm). |
| `d` | Show the **diff** introduced by this commit. |
| `s` | **Soft reset** to the highlighted commit (keeps changes staged). |
| `x` | **Mixed reset** (keeps changes in the working tree, unstages them). |
| `h` | **Hard reset** (drops the changes). |
| `r` | Refresh the log. |

> The number of log entries shown is limited by **Max log entries** in
> `Configuration → Git`. The default is reasonable for most repos;
> raise it if you work with very long histories.

---

## Tab 3: Branches

Lists local branches and remote-tracking branches.

- The current branch is marked with a green `*`.
- Remote-tracking branches are labelled `[remote]` and rendered in grey.

| Key | Effect |
| --- | --- |
| `Enter` | **Checkout** the highlighted local branch. |
| `n` | **New** branch (prompts for a name). |
| `d` / `Delete` | **Delete** the highlighted local branch (asks for confirmation; the current branch cannot be deleted). |
| `r` | **Rename** the highlighted local branch. |
| `m` | **Merge** the highlighted branch into the current one (asks for confirmation). |
| `r` | Refresh. |

---

## Tab 4: Stash

Lists the entries of `git stash list`.

| Key | Effect |
| --- | --- |
| `a` | **Apply** the highlighted stash (keeps it in the stack). |
| `p` / `Enter` | **Pop** the highlighted stash (apply and drop it). |
| `d` / `Delete` | **Drop** the highlighted stash entry. |

---

## Remote operations (any tab)

| Key | Effect |
| --- | --- |
| `f` | **Fetch** from the remote. |
| `l` | **Pull** (fetch + merge) from the active remote branch. |
| `u` | **Push** committed changes to the active remote branch. |

> Pairee uses the Git config in your repo (`.git/config` and
> `~/.gitconfig`). To override the author identity for a session, set
> **Author name** and **Author email** in
> `Configuration → Git`.

---

## Common pitfalls

- **Dashboard does not open**: the active panel is not inside a Git
  repository. Navigate one level up (`Backspace`) and try again, or
  enable `git_auto_detect`.
- **Push rejected**: the remote has new commits you do not have. Press
  `l` to pull (or rebase) first, resolve any conflicts, then `u`.
- **Stash conflicts on apply**: Pairee shows the conflicted files in
  the Status tab; resolve manually and `git add` them, then commit.

---

## Where to go next

- Configuration: [`41_reference_configuration`](41_reference_configuration.md) (Tab 6: Git Settings)
- Background workers: [`50_explanation_architecture`](50_explanation_architecture.md)
