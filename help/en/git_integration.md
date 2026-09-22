# Git Integration Reference Manual

Pairee features a fully integrated Git dashboard that allows you to monitor and manage your repository's state directly within the terminal UI. It leverages native libgit2 to provide fast, safe, and asynchronous repository operations.

---

## 1. Opening the Git Dashboard & Menu Actions

### 1.1 Dashboard Shortcut
* **Hotkey Shortcut:** Press **`Alt+G`** (or **`Alt+g`**) while inside any directory that belongs to a Git repository.
* **Top Menu:** Select **`Left Panel`** (or **`Right Panel`**) -> **`Git panel`**.

### 1.2 Repository Management from Explorer
* **Init Git repository (`GitInit`):** When in a non-repository directory, the panel menu offers **`Init Git repository`** to initialize a new Git repository at the current directory path.
* **Clone repository (`GitClone`):** Available from the panel menu (**`Clone repository...`**). Prompts for a remote repository URL (HTTPS or SSH) and an optional target directory name.
* **Auto-detection:** When `git_auto_detect` is enabled in configuration, Pairee automatically scans directories for `.git` folders as you browse.

### 1.3 Explorer Panel Visual Cues
* **Panel Title Indicator:** The active Git branch (e.g. `[git: main]`, `[git: feature/xyz]`, or `[git: (detached HEAD)]`) is displayed in the panel header.
* **File Status Badges & Colors:** Files with working tree changes are decorated with visual badges and colors:
  - `[M]` (Yellow): Modified file.
  - `[A]` (Green): Added / staged file.
  - `[?]` (Magenta): Untracked file.
  - `[D]` (Red): Deleted file.
  - `[!]` (Light Red): Conflicted file.

---

## 2. Interactive Git Dashboard Tabs

The Git dashboard popup features five distinct tabs. Use the **`Tab`** or **`Shift+Tab`** keys to cycle between them.

### 2.1 Status Tab (Pestaña de Estado)
Displays all changed, staged, unstaged, and conflicted files in your working directory.
* **File Status Indicators:**
  - `[staged]`: Staged changes in the index ready to be committed.
  - `[staged+]`: File has staged changes and additional unstaged modifications in the working tree.
  - `M` (Modified), `A` (Added), `D` (Deleted), `?` (Untracked), `R` (Renamed), `!` (Conflicted).
* **Keyboard Commands:**
  - **`Space`**: Toggles staging for the selected file (stages unstaged files, unstages staged files).
  - **`a`**: Stages all changed and untracked files (`git add -A`).
  - **`A`**: Unstages all files from the index.
  - **`x` / `Delete`**: Discards working tree changes in the selected file (prompts for confirmation).
  - **`i`**: Appends the selected file or pattern to `.gitignore`.
  - **`c`** (Commit): Opens the Commit dialog to commit staged changes.
    - Inside commit dialog: **`Ctrl+A`** toggles **Amend** (`--amend`) to revise the previous commit.
  - **`d`**: Opens the Git Diff viewer to inspect changes in the selected file.
  - **`s`**: Saves current changes to the stash stack (prompts for optional message, **`Ctrl+U`** toggles untracked files).
  - **`X`**: Aborts an in-progress merge if merge conflicts exist.
  - **`f` / `l` / `u`**: Fetch, Pull, or Push remote synchronization.
  - **`Esc`**: Closes the Git dashboard.

### 2.2 Log Tab (Historial)
Displays commit history of the active branch with incremental pagination and continuous scrolling.
* **Displayed Metadata Columns:**
  - **Commit Hash:** Shortened 7-character identifier.
  - **Date:** Commit timestamp formatted as `YYYY-MM-DD`.
  - **Author:** Author name.
  - **Message:** First line of the commit message.
* **Keyboard Commands:**
  - **`Enter`**: Checks out the selected commit in **detached HEAD** mode (with confirmation).
  - **`d`**: Opens the commit diff viewer showing changes introduced by the commit.
  - **`b` / `n`**: Creates a new branch pointing to the selected commit.
  - **`t`**: Creates a new tag pointing to the selected commit.
  - **`c`**: Cherry-picks the selected commit onto the current branch (with confirmation).
  - **`r`**: Reverts the selected commit by creating an inverse commit (with confirmation).
  - **`y`**: Copies the full 40-character commit SHA to the system clipboard.
  - **`s`**: Soft Reset to the selected commit (keeps working tree and index).
  - **`x`**: Mixed Reset to the selected commit (resets index, keeps working tree).
  - **`h`**: Hard Reset to the selected commit (discards all changes).
  - **`Esc`**: Closes the Git dashboard.

### 2.3 Branches Tab (Ramas)
Lists all local branches and remote-tracking branches with ahead/behind commit counters.
* **Display Indicators:**
  - Current active branch is marked with `*` and highlighted.
  - Remote-tracking branches are labeled with `[remote]` in gray.
  - Ahead/behind indicators: `[↑X ↓Y]` shows commits ahead of and behind the configured upstream.
* **Keyboard Commands:**
  - **`Enter`**:
    - On a local branch: switches to that branch.
    - On a remote branch: checks out by automatically creating a local branch tracking the remote.
  - **`n`**: Creates a new branch from HEAD.
  - **`d` / `Delete`**: Deletes the selected branch (works for local branches and remote branches on origin, with confirmation).
  - **`r`**: Renames the selected local branch.
  - **`b`**: Rebases the current branch onto the selected branch (with confirmation).
  - **`m`**: Merges the selected branch into the current branch (with confirmation).
  - **`R`**: Opens the **Remote Repositories** management dialog.
  - **`Esc`**: Closes the Git dashboard.

### 2.4 Stash Tab (Pila de Stash)
Lists all stashed working directory states.
* **Keyboard Commands:**
  - **`Enter` / `a`**: Applies the selected stash entry to your working directory.
  - **`p`**: Pops the selected stash entry (applies changes and drops it from the stash stack).
  - **`d`**: Opens the Diff viewer to inspect changes stored in the stash entry.
  - **`Delete` / `x`**: Drops the selected stash entry from the stack (with confirmation).
  - **`C`**: Clears all stash entries from the stack (with confirmation).
  - **`Esc`**: Closes the Git dashboard.

### 2.5 Tags Tab (Etiquetas)
Lists all repository tags (both lightweight and annotated).
* **Displayed Metadata Columns:**
  - **Tag Name:** Tag reference name.
  - **Commit:** Target commit hash.
  - **Annotation:** Optional annotated tag message.
* **Keyboard Commands:**
  - **`Enter`**: Checks out the tag in detached HEAD mode (with confirmation).
  - **`n`**: Creates a new tag for the HEAD commit (prompts for tag name).
  - **`d` / `Delete`**: Deletes the selected tag (with confirmation).
  - **`u`**: Pushes all repository tags to the remote repository.
  - **`Esc`**: Closes the Git dashboard.

---

## 3. Remote Operations & Authentication

### 3.1 Remote Synchronization
From any dashboard tab:
* **`f`**: Fetch updates from the upstream remote.
* **`l`**: Pull changes (fetch + merge/fast-forward) from the upstream remote branch.
* **`u`**: Push local branch commits to the remote branch (automatically sets `--set-upstream` when needed).

### 3.2 Multi-Remote Management Dialog (`R`)
Pressing **`R`** in the Branches tab opens the Remote Management dialog:
* View all configured remotes with their fetch and push URLs.
* **`a`**: Add a new remote (prompts for name and URL).
* **`d` / `Delete`**: Remove the selected remote (with confirmation).

### 3.3 Authentication Support
* **SSH Keys:** Automatically negotiates with active SSH agents, `~/.ssh/id_ed25519`, `~/.ssh/id_ecdsa`, and `~/.ssh/id_rsa`.
* **HTTPS:** Seamlessly interacts with system Git Credential Helpers (e.g. Git Credential Manager on Windows, macOS Keychain, and Linux libsecret).
