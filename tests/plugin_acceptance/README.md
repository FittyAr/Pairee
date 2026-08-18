# Plugin acceptance (CI)

Real plugin trees loaded by `src/plugin/acceptance.rs` during `cargo test`
(Check workflow on Linux and Windows).

| Plugin | What it proves |
|--------|----------------|
| `surface` | Stable `pairee.*` bindings exist; `_lua_api_version` matches Rust |
| `fs_roundtrip` | `mkdir` / `write` / `read` / `copy` / `rename` / `remove` / `File` |
| `cx_utils` | `pairee.cx` shape + `utils.target_os/hash/quote` |
| `command_echo` | `pairee.Command` builder and `:output()` (async) |

These are **not** shipped to users. They are the M3-style acceptance suite
for the APIs that exist today (fzf/zoxide ports still need those binaries).
