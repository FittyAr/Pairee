# Plugin acceptance (CI)

Real plugin trees loaded by `src/plugin/acceptance.rs` during `cargo test`
(Check workflow on Linux and Windows).

| Plugin | What it proves |
|--------|----------------|
| `surface` | Stable `pairee.*` bindings exist; `_lua_api_version` matches Rust |
| `fs_roundtrip` | `mkdir` / `write` / `read` / `copy` / `rename` / `remove` / `File` (incl. mime/mtime/hidden) |
| `cx_utils` | `pairee.cx` shape + `utils.target_os/hash/quote` |
| `command_echo` | `pairee.Command` builder and `:output()` (async) |
| `roadmap_api` | Untrusted plugin: `Path` / `Url` / `Cha` / `Error`, `fs.cha` jail, `pairee.state`, clipboard read refused, identity utils, `ui` widgets and layout, `rt` / `th`, `image.show` jail, `--- @since` / `--- @sync` |
| `fzf` | fzf port: `Command` with piped stdin/stdout, `write_all` / `close_stdin` / `wait_with_output`, `Path`, `cx`, `emit("cd")` (the test uses `sort` as the picker, so no fzf binary is needed) |
| `zoxide` | zoxide-like port in pure Lua: `ps.sub("on_cd")` with the sender metadata (fake visits from other plugins ignored), `pairee.state`, frecency ranking, `emit("cd")` |
| `code_preview` | Code previewer: `preview_code`, `Layout:split`, `Rect`, `Pad`, `Bar`, `Clear`, `Fill`, `:area`, a list of widgets from `peek`, `File` fields inherited from `Cha` |

These are **not** shipped to users. They are the acceptance suite for the
plugin API.
