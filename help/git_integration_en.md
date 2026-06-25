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

The Git dashboard popup displays three distinct tabs. Use the **`Tab`** key to switch between them.

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
  - **`c`** (Commit All): Stages all modified and untracked files (`git add -A`) and opens the Commit dialog.
  - **`r`** (Refresh): Queries the repository and re-reads the active status lists.
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
  - **`r`** (Refresh): Re-reads the commit log.

### 2.3 Branches Tab
Lists all local branches and remote-tracking branches available in the repository.
* **Display Indicators:**
  - The currently checked-out branch is marked with a green asterisk (`*`).
  - Remote-tracking branches are prefix-labeled with `[remote]` and rendered in gray.
* **Keyboard Commands:**
  - **`Enter`** (Checkout Branch): Switches HEAD to the selected local branch and updates your filesystem panel files. A confirmation dialog will appear first.
  - **`r`** (Refresh): Queries the repository for updated branches.

---

## 3. Commit Dialog & Author Override

When committing changes via the `Status` tab:
1. Press **`c`**. Pairee runs `git add -A` internally.
2. A prompt dialog appears: **"Commit All Changes"**.
3. Type your commit message. If empty, the commit is aborted.
4. Pairee resolves your identity as follows:
   - Queries settings `git_author_name` and `git_author_email`.
   - If those settings are blank, it falls back to querying the repository's local configuration (`.git/config`) or global user configurations (`~/.gitconfig`).
   - If no config is found, it uses the default fallback `Pairee User <pairee@localhost>`.
5. Press **`Enter`** to commit changes, or **`Esc`** to cancel.
