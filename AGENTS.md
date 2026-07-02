# Pairee Plugin Registry Branch

This is the orphan production branch **`plugin-registry`** of the Pairee repository. It is used exclusively to store, publish, and distribute the community public plugin catalog.

## Directory Structure

The structure of this branch is optimized to partition plugins and avoid single-folder collisions/heavy listings:

```text
plugin-registry/
├── .gitignore
├── AGENTS.md                           # These instructions
└── registry/
    ├── index.toml                      # Master catalog containing metadata and SHA-256 integrity hashes
    ├── blocklist.toml                  # Remote blocklist to disable/remove unsafe or broken plugins
    └── plugins/                        # Root folder for all plugins
        └── <author_initial_lowercase>/  # First letter of the author's name (e.g., 'f' for 'FittyAr', or '_' if not a-z)
            └── <author_name>/          # Author's folder (e.g., 'FittyAr')
                └── <plugin_name>/      # Plugin directory for the latest version
                    ├── manifest.toml   # Plugin manifest
                    ├── main.lua        # Main execution file
                    ├── sha256.sum      # SHA-256 checksums of the plugin files
                    ├── help/           # Help documentation
                    ├── lang/           # Localization/language files
                    └── screenshots/    # Screenshots and required icons
```

## Remote Blocklist (`registry/blocklist.toml`)

Maintainers can add plugins to the blocklist in case of critical issues, security vulnerabilities, or malicious behavior. 
Any plugin in this blocklist:
- Will be hidden from search, info, and remote listing.
- Cannot be installed or updated by users.
- If already installed, the local command check-updates or verification will flag/remove it.

Format of `registry/blocklist.toml`:
```toml
schema_version = "1"
generated_at = "2026-07-02T08:40:00Z"

[blocked]
# "malicious-plugin" = "Reason for block: contains unauthorized network requests"
```

## Rules for Agents (AI Coding Assistants)

When working on this branch or generating tools to interact with it, ensure you comply with the following guidelines:

1. **Plugin Location:** All plugin files must be packaged strictly into `registry/plugins/<author_initial_lowercase>/<author_name>/<plugin_name>/`.
2. **Registry Index (`index.toml`):** Whenever a plugin is added or updated, the corresponding entry in `registry/index.toml` must be updated, adhering to the serialized `RegistryIndex` structure.
3. **No Deletion (Append-Only):** Do not delete plugins or previous versions from the registry history, as the plugin database is cumulative.
4. **Emergency Blocklist:** To block/invalidate a plugin, add it to `registry/blocklist.toml` with a clear reason instead of deleting its files from the git history.
5. **Temporary Files:** Respect the `.gitignore` configured on this branch to avoid uploading build outputs (`target/`), example directories (`example/`), Cargo lockfiles (`Cargo.lock`), or editor temporary files.
