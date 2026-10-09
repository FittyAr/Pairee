# Coming from Another Program

Each preset keeps the keys of one family of programs. This page says what you
will find as you left it, what Pairee adds and what is different. The
complete lists are in the **Keys:** pages.

---

## Norton Commander, Far Manager, Midnight Commander → preset *Norton Commander / Far*

**As you know it.** The F-key row (`F1` help, `F2` user menu, `F3` view, `F4`
edit, `F5` copy, `F6` move, `F7` make folder, `F8` delete, `F9` menu, `F10`
quit, `F11` plugins, `F12` screens), `Shift+F4` new file, `Shift+F6` rename,
`Alt+F1`/`Alt+F2` drives, `Alt+F7` find, `Alt+F8` command history, `Alt+F10`
tree, `Alt+F11`/`Alt+F12` view and folder history, `Ctrl+F1`/`Ctrl+F2` panels,
`Ctrl+O`, `Ctrl+U`, `Ctrl+L`, `Ctrl+Q`, `Ctrl+P` hides the passive panel,
`Ctrl+B` the key bar, `Ctrl+H` hidden files, `Ctrl+R` reread, `Ctrl+A`
attributes, `Ctrl+G` apply command, `Ctrl+Z` describe, `Ctrl+W` tasks,
`Ctrl+F3`…`Ctrl+F12` sort, `Ctrl+1`…`Ctrl+9` views, `Insert` and the keypad
`+` `-` `*` select, `Ctrl+M` restores the selection, `Ctrl+\` goes to the root,
`Ctrl+PgUp`/`Ctrl+PgDn`. Letters go to the command line; `Ctrl+Enter`, `Ctrl+F`,
`Ctrl+[` and `Ctrl+]` insert names and paths, `Ctrl+E`/`Ctrl+X` walk the
history and `Ctrl+Y` clears it. `Alt` + a letter is the quick search.

**What Pairee adds** is on `Ctrl+Alt` + a letter, so that `Alt` + letters stay
free for the quick search: `T` / `W` new / close tab, `O` open in a new tab,
`G` Git, `R` SSH, `S` folder sizes, `D` disk usage, `B` hotlist, `F` quick
filter. `Alt+1`…`Alt+9` and `Alt+PgUp`/`Alt+PgDn` switch tabs, `Alt+Backspace`
undoes a file operation (`Alt+Shift+Backspace` redoes it), `Ctrl+Insert` copies
names, `Alt+Shift+Insert` paths, and `Ctrl+K` lists every key.

**Different.** `Ctrl+Shift+F6` is the multi-rename tool and `Ctrl+T` the
transfer panel (Far's tree panel is `Alt+F10`). `Shift+Delete` deletes without
the recycle bin. On Windows, `Ctrl+Alt` is `AltGr` and some layouts type a
character with it; every one of these actions is also in the menu (`F9`).

---

## Windows Explorer, VS Code, Total Commander → preset *Standard*

**As you know it.** `Ctrl+C` / `Ctrl+X` / `Ctrl+V` copy, cut and paste files,
`Delete` sends to the recycle bin and `Shift+Delete` deletes, `F2` renames,
`Ctrl+Z` / `Ctrl+Y` undo and redo, `Ctrl+A` selects all, `Ctrl+F` searches the
folder, `Alt+←` / `Alt+→` go back and forward, `Alt+↑` and `Backspace` go up,
`Ctrl+T` / `Ctrl+W` / `Ctrl+Tab` handle tabs, `Ctrl+Shift+N` makes a folder,
`Ctrl+N` a file, `Alt+Enter` shows properties, `Ctrl+Shift+P` opens the command
palette, `Ctrl+,` the settings, `` Ctrl+` `` the command line, `Ctrl+K Ctrl+S`
the shortcuts list, `Ctrl+Q` quits. Typing a name jumps to the file.

**From Total Commander,** because Pairee has two panels: `F3` views, `F4`
edits, `F5` copies and `F6` moves to the other panel, `F7` makes a folder,
`F8` deletes, `Ctrl+M` is the multi-rename tool and `Ctrl+U` swaps the panels.

**Different.** `F5` copies instead of refreshing: folders refresh by
themselves, and `Ctrl+R` rereads. Find next is `Ctrl+G`, since `F3` views.
`F10` opens the menu bar. Folder tabs use `Ctrl+Tab`, so screens (editor,
viewer) use `F12` and `Ctrl+F12`. `Ctrl+Shift` + a letter needs a terminal
that reports it; each such action has another key (`Ctrl+Alt+F` finds files,
`F7` makes a folder, `Alt+C` copies the path).

---

## Vim, Neovim, vifm, oil.nvim, nvim-tree → preset *Neovim*

**As you know it.** `h` `j` `k` `l`, `gg` / `G`, `Ctrl+U` / `Ctrl+D`,
`Ctrl+B` / `Ctrl+F`, `/` `n` `N`, `v` / `V` visual selection, `u` / `Ctrl+R`
undo and redo, `gt` / `gT` tabs, `Ctrl+W` `w`/`h`/`l`/`x`/`o` move between,
swap and hide panels like windows, `Ctrl+O` / `Ctrl+I` go back and forward,
`K` shows details, `ZZ` quits, `g?` lists the keys. The leader is `Space`, as
in LazyVim: `Space f f` finds files, `Space f r` recent folders, `Space g g`
Git, `Space ?` keys.

**From file managers:** `yy` yanks, `p` pastes a copy and `P` pastes
overwriting, `x` cuts, `dd` sends to the trash and `DD` deletes, `cw` renames
(`cW` only the name), `a` creates a file (a folder when the name ends with
`/`), `-` goes up (vim-vinegar, oil.nvim), `g.` or `za` shows hidden files,
`t` tags (vifm), `gx` opens the context menu (open with...), `yp` / `yn` copy the path / name.

**Different.** `:` opens the command palette (search an action by name) and
`!` the shell command line. Counts (`5j`) are not supported. The built-in
editor is not modal: it keeps the usual editing keys and `Esc` closes it.
`Ctrl+I` arrives as `Tab` on older terminals. The Norton F-keys still work
(`F5` copy, `F7` folder...).

---

## yazi → preset *yazi*

**As you know it.** `h` `j` `k` `l`, `H` / `L` back and forward, `gg` / `G`,
`Ctrl+U` / `Ctrl+D` / `Ctrl+B` / `Ctrl+F`, `Space` selects and moves down,
`v` / `V` visual, `Ctrl+A` all, `Ctrl+R` inverts, `y` / `x` / `p` / `P` yank,
cut, paste and paste overwriting, `Y` / `X` drop the yank, `-` links, `d` / `D`
trash and delete, `a` creates (a trailing `/` makes a folder), `r` renames,
`.` hidden files, `f` filters, `/` `n` `N` search, `s` / `S` search files and
contents, `z` finds a file, `Z` jumps to a folder you visited, `cc` / `cd` /
`cf` / `cn` copy path, folder, name and name without extension, `,` + a
letter sorts, `m` + a letter changes the columns, `t` new tab, `1`…`9` switch
tab, `[` / `]` previous / next, `{` / `}` move it, `Ctrl+C` closes it, `w`
tasks, `;` / `:` shell, `~` keys, `q` / `Q` quit, `o` / `l` / `Enter` open,
`O` the context menu (open with...). Key sequences wait for their next key.

**Different.** Pairee shows two panels: `Tab` switches between them and `i`
shows the information panel (yazi's "spot"); `Ctrl+Q` previews the file in the
other panel. Sorting in reverse is `, r` instead of the uppercase letters.
There is one kind of link (`_` is not used). `e` edits with the built-in
editor. `z` and `Z` use Pairee's own search and folder history instead of
fzf and zoxide. The Norton F-keys still work (`F5` copy, `F7` folder...).
