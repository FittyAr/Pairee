# Neovim preset — keyboard reference

This page lists the keys the preset ships with; your own changes are not shown here. Press `Space ?` in Pairee for the keyboard shortcuts list, where you can search every key and change it.

- Letters are commands; a letter without a command does nothing.
- The leader key, written <leader>, is `Space`.

## Panels

### Navigation

| Keys | Action |
| :--- | :--- |
| `Ctrl+o` | Back |
| `Alt+F1` | Change drive (left panel) |
| `Alt+F2` | Change drive (right panel) |
| `Ctrl+PageDown` | Enter folder or archive |
| `Ctrl+Alt+b` | Folder hotlist |
| `Alt+F10` | Folder tree |
| `Ctrl+i` | Forward |
| `Home` · `g g` | Go to first item |
| `Ctrl+Alt+1` | Go to folder shortcut 1 |
| `Ctrl+Alt+2` | Go to folder shortcut 2 |
| `Ctrl+Alt+3` | Go to folder shortcut 3 |
| `Ctrl+Alt+4` | Go to folder shortcut 4 |
| `Ctrl+Alt+5` | Go to folder shortcut 5 |
| `Ctrl+Alt+6` | Go to folder shortcut 6 |
| `Ctrl+Alt+7` | Go to folder shortcut 7 |
| `Ctrl+Alt+8` | Go to folder shortcut 8 |
| `Ctrl+Alt+9` | Go to folder shortcut 9 |
| `g h` | Go to home folder |
| `End` · `G` | Go to last item |
| `-` · `Backspace` · `Ctrl+PageUp` · `h` | Go to parent folder |
| `Ctrl+\` · `g /` | Go to root folder |
| `Ctrl+d` | Half page down |
| `Ctrl+u` | Half page up |
| `Down` · `j` | Move down |
| `Up` · `k` | Move up |
| `Enter` · `l` | Open / run |
| `Ctrl+f` · `PageDown` | Page down |
| `Ctrl+b` · `PageUp` | Page up |

### Selection

| Keys | Action |
| :--- | :--- |
| `*` · `Space i` | Invert selection |
| `Ctrl+m` | Restore selection |
| `Insert` · `t` | Select / unselect item |
| `Shift+Plus` · `Space a` | Select all |
| `Plus` · `Space Plus` | Select group |
| `Space A` | Unselect all |
| `Space -` | Unselect group |
| `V` · `v` | Visual selection |

### Files

| Keys | Action |
| :--- | :--- |
| `Ctrl+g` | Apply command |
| `Ctrl+Alt+s` | Calculate folder sizes |
| `F5` | Copy to other panel |
| `a` | Create file or folder |
| `Alt+F6` | Create link |
| `F8` | Delete |
| `D D` · `Shift+Delete` | Delete permanently |
| `Ctrl+z` | Describe file |
| `Ctrl+Alt+d` | Disk usage |
| `F4` · `e` | Edit file |
| `Ctrl+a` · `Space .` | File attributes |
| `F7` | Make folder |
| `F6` | Move to other panel |
| `d d` | Move to recycle bin |
| `Ctrl+Shift+F6` · `Space r` | Multi-rename |
| `Shift+F4` | New file |
| `Alt+F5` | Print file |
| `Ctrl+r` · `Alt+Shift+Backspace` | Redo file operation |
| `Shift+F6` · `c w` | Rename |
| `c W` | Rename (name only) |
| `Alt+Backspace` · `u` | Undo file operation |
| `F3` · `o` | View file |
| `Alt+F3` | View file (alternative) |
| `Alt+Delete` | Wipe file |

### Clipboard

| Keys | Action |
| :--- | :--- |
| `Space y` | Clear file clipboard |
| `Ctrl+Insert` · `y n` | Copy name |
| `Alt+Shift+Insert` · `y p` | Copy path to clipboard |
| `x` | Cut |
| `p` | Paste |
| `P` | Paste, overwriting |
| `y y` | Yank (copy to clipboard) |

### Archives

| Keys | Action |
| :--- | :--- |
| `Shift+F3` | Archive commands |
| `Shift+F1` · `Space z` | Compress files |
| `Shift+F2` · `Space x` | Extract archive |

### Search

| Keys | Action |
| :--- | :--- |
| `Alt+F8` | Command history |
| `Alt+F11` | File view history |
| `Alt+F7` · `Space f f` | Find files |
| `Alt+F12` · `Space f r` | Folder history |
| `n` | Next match |
| `Space F` | Panel filter |
| `N` | Previous match |
| `Ctrl+Alt+f` · `Space /` | Quick filter |
| `/` | Search in panel |

### View

| Keys | Action |
| :--- | :--- |
| `Ctrl+Alt+k` | Cycle key bar modifiers |
| `Ctrl+l` | Refresh |
| `g .` · `z a` · `Ctrl+h` | Show / hide hidden files |
| `Ctrl+n` | Toggle long names |
| `Alt+F9` | Video mode |
| `Space v 9` · `Ctrl+9` | View: alternative full |
| `Space v 1` · `Ctrl+1` | View: brief |
| `Space v 6` · `Ctrl+6` | View: descriptions |
| `Space v 5` · `Ctrl+5` | View: detailed |
| `Space v 3` · `Ctrl+3` | View: full |
| `Space v 8` · `Ctrl+8` | View: links |
| `Space v 2` · `Ctrl+2` | View: medium |
| `Space v 7` · `Ctrl+7` | View: owners |
| `Space v 4` · `Ctrl+4` | View: wide |

### Sort

| Keys | Action |
| :--- | :--- |
| `g s r` | Reverse sort order |
| `Ctrl+F9` | Sort by access time |
| `Ctrl+F8` · `g s c` | Sort by creation time |
| `Ctrl+F10` | Sort by description |
| `Ctrl+F4` · `g s e` | Sort by extension |
| `Ctrl+F5` · `g s m` | Sort by modification time |
| `Ctrl+F3` · `g s n` | Sort by name |
| `Ctrl+F11` | Sort by owner |
| `Ctrl+F6` · `g s s` | Sort by size |
| `Ctrl+F12` | Sort modes |
| `Ctrl+F7` | Unsorted |

### Panels

| Keys | Action |
| :--- | :--- |
| `Ctrl+w h` | Focus left panel |
| `Ctrl+w l` | Focus right panel |
| `K` | Information panel |
| `Ctrl+q` · `g p` | Quick view |
| `Ctrl+w o` | Show / hide both panels |
| `Ctrl+p` | Show / hide inactive panel |
| `Ctrl+F1` | Show / hide left panel |
| `Ctrl+F2` | Show / hide right panel |
| `Ctrl+w x` | Swap panels |
| `Ctrl+w w` · `Tab` | Switch panel |
| `Ctrl+t` | Transfer panel |

### Tabs and screens

| Keys | Action |
| :--- | :--- |
| `Ctrl+Alt+w` · `Space t c` | Close tab |
| `Alt+1` | Go to tab 1 |
| `Alt+2` | Go to tab 2 |
| `Alt+3` | Go to tab 3 |
| `Alt+4` | Go to tab 4 |
| `Alt+5` | Go to tab 5 |
| `Alt+6` | Go to tab 6 |
| `Alt+7` | Go to tab 7 |
| `Alt+8` | Go to tab 8 |
| `Alt+9` | Go to tab 9 |
| `Alt+Shift+PageUp` · `Space t <` | Move tab left |
| `Alt+Shift+PageDown` · `Space t >` | Move tab right |
| `Ctrl+Alt+t` · `Space t n` | New tab |
| `Ctrl+Tab` | Next screen |
| `Alt+PageDown` · `Alt+Right` · `g t` | Next tab |
| `Ctrl+Alt+o` | Open in new tab |
| `Ctrl+Shift+Tab` | Previous screen |
| `Alt+Left` · `Alt+PageUp` · `g T` | Previous tab |
| `F12` | Screens list |

### Git

| Keys | Action |
| :--- | :--- |
| `Ctrl+Alt+g` · `Space g g` | Git panel |

### Remote

| Keys | Action |
| :--- | :--- |
| `Ctrl+Alt+r` · `Space s s` | SSH: connect |

### Tools

| Keys | Action |
| :--- | :--- |
| `Ctrl+y` | Clear command line |
| `!` | Command line |
| `Ctrl+[` | Left folder to command line |
| `Ctrl+Enter` | Name to command line |
| `Ctrl+x` | Next command |
| `F11` | Plugins |
| `Ctrl+e` | Previous command |
| `Ctrl+]` | Right folder to command line |
| `Space w` | Task list |
| `F2` · `Space u` | User menu |

### System

| Keys | Action |
| :--- | :--- |
| `Esc` | Cancel / close |
| `:` · `Ctrl+Alt+p` · `Ctrl+P` | Command palette |
| `Menu` · `Shift+F10` · `g x` · `Ctrl+Alt+m` | Context menu |
| `F1` | Help |
| `Space ?` · `g ?` · `Ctrl+K` | Keyboard shortcuts |
| `F9` | Menu bar |
| `F10` · `Space q q` · `Z Z` | Quit |
| `Shift+F9` | Save setup |
| `Space ,` | Settings |

## Editor

### Selection

| Keys | Action |
| :--- | :--- |
| `Ctrl+b` | Block selection mode |
| `Ctrl+a` · `Ctrl+A` | Select all |

### Files

| Keys | Action |
| :--- | :--- |
| `F8` | Discard changes |
| `Ctrl+y` · `Ctrl+Z` | Redo |
| `Ctrl+d` · `Ctrl+r` | Reload from disk |
| `Ctrl+s` · `F2` | Save |
| `Shift+F2` | Save as |
| `Ctrl+z` | Undo |

### Clipboard

| Keys | Action |
| :--- | :--- |
| `Ctrl+Insert` · `Ctrl+c` · `Ctrl+C` | Copy |
| `Ctrl+x` · `Shift+Delete` · `Ctrl+X` | Cut |
| `Ctrl+v` · `Shift+Insert` · `Ctrl+V` | Paste |

### Search

| Keys | Action |
| :--- | :--- |
| `Ctrl+f` · `F7` | Search |
| `F3` · `Shift+F7` | Search next |

### View

| Keys | Action |
| :--- | :--- |
| `F4` | Open in viewer |

### System

| Keys | Action |
| :--- | :--- |
| `Esc` · `F10` | Close editor |

## Viewer

### Navigation

| Keys | Action |
| :--- | :--- |
| `End` · `G` | Go to end |
| `Home` · `g g` | Go to start |
| `Down` · `j` | Line down |
| `Up` · `k` | Line up |
| `Ctrl+d` · `Ctrl+f` · `PageDown` | Page down |
| `Ctrl+b` · `Ctrl+u` · `PageUp` | Page up |

### Files

| Keys | Action |
| :--- | :--- |
| `F6` | Edit file |

### Search

| Keys | Action |
| :--- | :--- |
| `/` · `F7` | Search |
| `F3` · `n` | Search next |

### View

| Keys | Action |
| :--- | :--- |
| `F8` | Encoding |
| `F4` | Text / hex view |

### System

| Keys | Action |
| :--- | :--- |
| `Esc` · `F10` · `q` | Close viewer |

## Lists

### Navigation

| Keys | Action |
| :--- | :--- |
| `Enter` | Choose |
| `Down` · `j` | Down |
| `Home` · `g g` | First item |
| `End` · `G` | Last item |
| `Ctrl+d` · `Ctrl+f` · `PageDown` | Page down |
| `Ctrl+b` · `Ctrl+u` · `PageUp` | Page up |
| `Up` · `k` | Up |

### System

| Keys | Action |
| :--- | :--- |
| `Esc` · `q` | Close |
