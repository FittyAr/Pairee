# Pairee Lua API (versioned)

This is the **semver contract** for plugins. Crate version (`0.7.x`) can move
without a Lua major bump if `pairee.*` stays compatible.

| Item | Location |
|------|----------|
| Current version | **1.0.0** (`pairee._lua_api_version`) |
| Surface inventory | [v1.md](./v1.md) |
| Lua-only changelog | [CHANGELOG.md](./CHANGELOG.md) |
| How-to | [plugin-dev-guide.md](../../plugin-dev-guide.md) |
| In-app help | [help/en/plugins.md](../../../help/en/plugins.md) |
| CI plugins | [tests/plugin_acceptance/](../../../tests/plugin_acceptance/) |

A plugin can refuse to load on an older host:

```lua
local v = pairee._lua_api_version or "0.0.0"
assert(v:sub(1, 2) == "1.", "needs Lua API 1.x")
```
