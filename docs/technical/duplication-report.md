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

| | Before (`dee133e`) | After | G.5 (`7d1da5e`) | Change since before |
|---|---:|---:|---:|---:|
| Clone pairs | 265 | 77 | 59 | -78% |
| Duplicated lines | 4,286 | 824 | 591 | -86% |
| Duplicated lines (%) | 6.68% | 1.39% | 0.73% | -5.95 pts |
| Duplicated tokens (%) | 6.95% | 1.53% | 0.82% | -6.13 pts |
| Lines of Rust (src + tests) | 64,191 | 59,142 | 80,892 | +16,701 |
| `dialogs.top().cloned()` (clone dialog per key) | 51 | 3 | 3 | |

The "before" column is the branch point; the editor work that landed in
master later (`938b96d`) only touched a few of the clusters listed here.
The G.5 column was measured after the Fase G waves (VFS, archives, Git
jobs, tabs, sessions, smoke tests) and the `too_many_lines` pass: the code
grew by about 21,700 lines while the duplicated lines went down.

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
- `clippy::too_many_lines` is enabled with `too-many-lines-threshold = 100`
  (`[lints.clippy]` in `Cargo.toml`, so CI's `-D warnings` enforces it on
  every target). The 38 functions above it were split in G.5, with no
  `#[allow]` left: request and action dispatchers route to per-group
  handlers (plugin requests, navigation, UI settings, tools, transfer
  queue `QueueOp`), long renderers became line builders plus small render
  steps, the CLI subcommands moved to `run::cli`, `AppConfig` loading to
  `config::loading`, and data that was code became tables (action names,
  Yazi sort/view keys, About libraries, developer-tool options).

## What is left

Remaining clone pairs of 13 lines or more (none above 15):

| Lines | Blocks | Files |
|---:|---:|---|
| 15 | 1 | `ui_settings/plugins.rs` ↔ `plugin_menu/dev/actions/build.rs` |
| 14 | 1 | `fs/archive/tar_ops.rs` ↔ `zip_ops.rs` |
| 14 | 1 | `ui/tests_archive.rs` ↔ `ui/tests_disk_usage.rs` (test setup) |
| 13 | 2 | `plugin_menu/dev/actions/build.rs` (internal) |
| 13 | 3 | `git_new_popups/confirm.rs` ↔ `diff.rs` ↔ `remote_manage.rs` (layout) |
| 13 | 1 | `ui/popup/git_panel/lines.rs` (internal) |
| 13 | 1 | `config/localization/loader.rs` (internal) |
| 13 | 1 | `input_popup/command_palette.rs` ↔ `which_key.rs` (imports) |
| 13 | 1 | `ui/tests_disk_usage.rs` ↔ `popup/sync_dirs/tests.rs` (test setup) |

The rest are 8 to 12 line pairs, mostly import blocks and small popup
layouts (a centered box, a title, a hint line) that already use the kit
and differ only in content.

Removed in G.5: the panel test fixtures and the totals of the panel footer
and info panel (`PanelState::entry_totals`), the Git confirm/diff/checkout
frames (`kit::frame_in`), the confirm and wipe prompts (`kit::TextBox`),
the `make_tool` test helper of the command policy, and the internal clones
of `transfer_panel/queue.rs`, `plugin/runtime/bindings/app.rs`, the
plugin-manager tabs (`plugin_menu/detail.rs`) and the Yazi popups.
