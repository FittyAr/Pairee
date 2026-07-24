# Git Integration Reference Manual

Pairee features a fully integrated Git dashboard that allows you to monitor and manage your repository's state directly within the terminal UI. It leverages the native Git library to provide fast, safe, and asynchronous repository operations.

---

## 1. Opening the Git Dashboard

To launch the Git Integration dashboard, your active file panel must be inside a folder that is part of a valid Git repository.
* **Hotkey Shortcut:** Press **`Alt+G`** (or **`Alt+g`**).
* **Dropdown Menu:** Select **`Left Panel`** (or **`Right Panel`**) -> **`Git`** from the top menu.
* **Auto-detection:** If `git_auto_detect` is enabled in configuration, Pairee will automatically scan directories for `.git` folders as you browse.

---

## 2. Interactive Git Dashboard Tabs

The Git dashboard popup displays four distinct tabs. Use the **`Tab`** key (or **`Shift+Tab`**) to switch between them.

### 2.1 Status Tab
This tab displays all unstaged, staged, and conflicted files in your working directory.
* **File Prefix Indicators:**
  - `M` (Yellow) ⋄ **Modified:** The file has been modified in the working tree.
  - `A` (Green) ⋄ **Added:** The file is newly created and staged in the index.
  - `D` (Red) ⋄ **Deleted:** The file has been deleted from the repository.
  - `?` (Dark Gray) ⋄ **Untracked:** The file is new and not yet tracked by Git.
  - `R` (Cyan) ⋄ **Renamed:** The file has been renamed.
  - `!` (Magenta) ⋄ **Conflicted:** The file has unresolved merge conflicts.
* **Keyboard Commands:**
  - **`Space`**: Toggles staging for the selected file (stages unstaged files, unstages staged files).
  - **`c`** (Commit All): Opens the Commit dialog to commit prepared changes.
  - **`d`**: Opens the Git Diff viewer to inspect changes in the selected file.
  - **`s`**: Saves the current changes to the stash stack (prompts for an optional message).
  - **`r`** (Refresh): Re-reads active status lists.
  - **`Esc`**: Closes the Git dashboard.

### 2.2 Log Tab
Displays a detailed commit history of the active branch, starting from `HEAD` down to the configured log limit.
* **Displayed Metadata Columns:**
  - **Commit Hash:** Shortened 7-character hexadecimal identifier.
  - **Date:** Commit timestamp formatted as `YYYY-MM-DD`.
  - **Author:** The name of the developer who made the commit.
  - **Message:** The first line of the commit message.
* **Keyboard Commands:**
  - **`Enter`** (Checkout Commit): Checks out the highlighted commit, placing your repository into a **detached HEAD** state. A confirmation dialog will appear first.
  - **`d`**: Opens the commit diff to inspect changes introduced by this commit.
  - **`s`**: Performs a **Soft Reset** to the selected commit.
  - **`x`**: Performs a **Mixed Reset** to the selected commit.
  - **`h`**: Performs a **Hard Reset** to the selected commit.
  - **`r`** (Refresh): Re-reads the commit log.

### 2.3 Branches Tab
Lists all local branches and remote-tracking branches available in the repository.
* **Display Indicators:**
  - The currently checked-out branch is marked with a green asterisk (`*`).
  - Remote-tracking branches are prefix-labeled with `[remote]` and rendered in gray.
* **Keyboard Commands:**
  - **`Enter`** (Checkout Branch): Switches HEAD to the selected local branch.
  - **`n`**: Prompts for a new branch name and creates it.
  - **`d` / `Delete`**: Deletes the selected local branch (requires confirmation; current branch cannot be deleted).
  - **`r`**: Prompts for a new name to rename the selected local branch.
  - **`m`**: Merges the selected branch into the current branch (requires confirmation).
  - **`r`** (Refresh): Re-reads branches.

### 2.4 Stash Tab
Lists all stashed changes in the repository stack.
* **Keyboard Commands:**
  - **`a`**: Applies the selected stash entry changes back to your working directory.
  - **`p` / `Enter`**: Pops the selected stash entry (applies changes and drops it from the stash stack).
  - **`d` / `Delete`**: Drops the selected stash entry from the stack.

---

## 3. Remote Operations

From any tab of the Git Dashboard, you can perform remote synchronization:
* **`f`**: Fetch changes from the remote repository.
* **`l`**: Pull changes (fetch and merge) from the active remote branch.
* **`u`**: Push committed changes to the remote branch.
