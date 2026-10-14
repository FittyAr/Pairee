# Help for PLUGIN_NAME Plugin

This is the help documentation for your Pairee plugin.

## Usage

Describe how to use your plugin here.

## Commands

| Command | Default keys | What it does |
|---------|--------------|--------------|
| Say hello | `Ctrl+Alt+e h` (Neovim `<leader>ph`, yazi `Alt+p h`) | Greets the hovered file |

Every command is also in the command palette. Rebind it in
`keybindings.toml` with its id, `plugin.PLUGIN_NAME.hello`.

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `greet_on_setup` | `false` | Log a greeting when the plugin loads |

## Permissions

If your plugin starts external programs with `pairee.Command` or
`pairee.fs.spawn`, set `requires_trust = true` and declare every program in
`manifest.toml`:

```toml
[permissions]
commands = ["git", "rg"]
```

In Secure Mode only these commands may run. They must be bare names (no
paths) found on `PATH`. Shells, interpreters, network clients and command
wrappers (`sh`, `bash`, `cmd`, `powershell`, `python`, `node`, `curl`,
`env`, `xargs`, ...) are always refused, even when declared, so call the
tool you need directly instead of going through a shell.
