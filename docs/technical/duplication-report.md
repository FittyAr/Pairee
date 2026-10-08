# Code duplication report

How duplicated code was measured, what was removed, and what is left.

## How it is measured

[jscpd](https://github.com/kucherenko/jscpd) 4 over `src` and `tests`, Rust
only, with a clone counted when at least 8 lines and 60 tokens repeat:

```sh
npx jscpd@4 --min-lines 8 --min-tokens 60 --format rust \
    --reporters json --output target/jscpd src tests
```

`target/jscpd/jscpd-report.json` lists every clone pair; `statistics.total`
holds the totals below.

## Before and after

| | Before (`dee133e`) | After | Change |
|---|---:|---:|---:|
| Clone pairs | 265 | 77 | -71% |
| Duplicated lines | 4,286 | 824 | -81% |
| Duplicated lines (%) | 6.68% | 1.39% | -5.29 pts |
| Duplicated tokens (%) | 6.95% | 1.53% | -5.42 pts |
| Lines of Rust (src + tests) | 64,191 | 59,142 | -5,049 |
| `dialogs.top().cloned()` (clone dialog per key) | 51 | 3 | |

The "before" column is the branch point; the editor work that landed in
master later (`938b96d`) only touched a few of the clusters listed here.

## Biggest clusters before

Duplicated lines, number of clone blocks, files.

| Lines | Blocks | Files |
|---:|---:|---|
| 270 | 7 | `input_popup/copy/mod.rs` ↔ `input_popup/rename_move/mod.rs` |
| 130 | 1 | `prompts/file_ops/copy/options.rs` ↔ `rename_move/options.rs` |
| 111 | 4 | `prompts/file_ops/copy/mod.rs` ↔ `rename_move/mod.rs` |
| 108 | 1 | `prompts/file_ops/copy/flags.rs` ↔ `rename_move/flags.rs` |
| 102 | 5 | `git_new_popups/prompts.rs` (internal) |
| 95 | 7 | `history_list/handlers.rs` (internal) |
| 88 | 1 | `ui/popup/editor/mod.rs` ↔ `ui/popup/viewer.rs` (search dialog) |
| 88 | 4 | `ui/menu/left.rs` ↔ `ui/menu/right.rs` |
| 74 | 1 | `prompts/file_ops/copy/input.rs` ↔ `rename_move/input.rs` |
| 73 | 1 | `ui/popup/editor/highlight.rs` ↔ `ui/viewer/text.rs` |
| 70 | 2 | `ui/quickview/img_render.rs` ↔ `ui/viewer/image.rs` |
| 62 | 4 | `history_lists/lists.rs` ↔ `history_lists/tree.rs` |
| 62 | 2 | `ui/panel/file_links.rs` ↔ `ui/panel/medium.rs` (and 5 more view modes) |
| 61 | 2 | `prompts/help/markdown.rs` ↔ `update/wrap.rs` |
| 59 | 2 | `input_popup/color_groups.rs` ↔ `files_highlighting.rs` |
| 58 | 4 | `transfer/worker/copy_phase` ↔ `delete_phase.rs` |
| 53 | 4 | `git_new_popups/confirm.rs` (internal) |
| 51 | 4 | `ui/popup/command_palette.rs` ↔ `which_key.rs` |

## What replaced them

Shared building blocks (each used by several dialogs):

- **`app::text_input::TextField`**: one grapheme-aware text field (cursor,
  Home/End, Delete, paste, AltGr) for every prompt instead of ad-hoc
  `String` + cursor code.
- **`app::form::FormLayout`**: focus order, Tab/arrow movement and field
  editing for form dialogs (copy/move, mkdir, rename, search, SSH...).
- **`app::list_nav`**: `list_key`, `filter_list_key` and `scroll_key` for
  every list or scrolling popup.
- **`ui::popup::kit`**: frames, prompt text, input boxes, button bars,
  checkbox rows, `ListPopup`, `list_area` and `FilterListView`.
- **`fs::transfer::control::JobControl`**: progress events, cancel/pause and
  per-file bookkeeping shared by every transfer worker and backend.
- **Table-driven code**: config dialog rows, Git confirm actions, Git panel
  tab actions, menus (`MenuBuilder`), panel view columns, theme color
  properties, transfer job-log events.

Merged duplicates: Copy and Move dialogs became one `TransferPrompt`; the
viewer and editor share one search dialog and one search highlighter; the
quick view and viewer share one image renderer; help, release notes and
About share `wrap_lines`; color groups and file highlighting share one
`ColorList` dialog; command palette and which-key share one filter list;
the left and right menus are one menu built per side.

Dialog handlers now edit the dialog in place through `dialogs.top_mut()`
instead of cloning it on every key and replacing it afterwards.

Later (G.2): the ad-hoc filesystem traits `fs::du::DuSource`,
`multi_rename::RenameBackend` and `ssh::RemoteFs`, the SFTP `walk_dir` /
`read_directory` copies and the "SSH or local" branches of the SSH copy
backend were replaced by one port, `fs::vfs::Vfs`; the zip, tar and 7z
extractors (and the `tar_ops.rs` ↔ `zip_ops.rs` pair below) became one
generic extractor over `fs::archive::format::ArchiveReader`.

## Clippy limits

- `too-many-arguments-threshold` is now 7 (was 12). Functions above it take
  a parameter struct: `ListOptions` (directory listings), `ScrollView` and
  `ScrollTarget` (scrollbars), `ViewerOpts`, `Pane` (plugin manager tabs),
  `JobControl`, among others.
- `clippy::too_many_lines` (100) is not enabled: 46 functions still exceed
  it. Most are large `match` dispatchers (plugin API dispatcher, action
  dispatch in `run.rs` and `ui_settings`, key handlers) where splitting
  would scatter one table over several functions. Turning it on needs a
  dedicated pass; the list is in the next section.

## What is left

Remaining clone pairs of 15 lines or more:

| Lines | Blocks | Files |
|---:|---:|---|
| 26 | 2 | `plugin_menu/dev/actions/build.rs` (internal) |
| 24 | 2 | `git_panel/tabs/status_log.rs` (internal) |
| 22 | 1 | `transfer_panel/queue.rs` (internal) |
| 22 | 1 | `app/state/panel.rs` (internal) |
| 20 | 2 | `plugin/runtime/bindings/app.rs` (internal) |
| 20 | 2 | `ui/popup/mod.rs` (internal) |
| 19 | 2 | `ui/popup/menus/dialogs.rs` (internal) |
| 19 | 1 | `git_new_popups/confirm.rs` ↔ `git_panel/mod.rs` |
| 18 | 1 | `plugin/command_policy/mod.rs` ↔ `resolve.rs` |
| 17 | 1 | `git_new_popups/diff.rs` ↔ `git_panel/mod.rs` |
| 16 | 1 | `prompts/confirm.rs` ↔ `prompts/file_ops/wipe.rs` |
| 16 | 1 | `fs/archive/tar_ops.rs` ↔ `zip_ops.rs` |
| 15 | 1 | `ui_settings/plugins.rs` ↔ `plugin_menu/dev/actions/build.rs` |

The rest are 10 to 13 line pairs, mostly small popup layouts (a centered
box, a title, a hint line) that already use the kit and differ only in
content.

Functions over 100 lines (`clippy::too_many_lines`), longest first:
`plugin/manager/dispatcher.rs` (230), `run.rs` (205),
`ui/popup/plugin_menu/dev.rs` (196), `actions/ui_settings/tools.rs` (194),
`ui/popup/update/mod.rs` (184), `git_panel/tabs/status_log.rs` (181),
`transfer_panel/queue.rs` (178), `ui/popup/git_panel/mod.rs` (177), then 38
more between 100 and 170 lines.
