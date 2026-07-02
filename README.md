# Pairee Plugin Registry

Welcome to the official community plugin repository for **Pairee**, a highly extensible terminal-based file manager. 

This repository (specifically the `plugin-registry` branch) contains the public index and source manifests for community-curated Pairee plugins. 

---

## 🚀 How to Use Plugins in Pairee

Pairee has a built-in plugin manager CLI (`pairee plugin`) that interacts directly with this registry. You do not need to clone this branch manually to install plugins.

### 🔍 Search for Plugins
To search the public catalog for available plugins:
```bash
pairee plugin search <query>
```

### 📦 Install a Plugin
To download and install a plugin from this registry:
```bash
pairee plugin install <plugin_name>
```

### 📋 List Installed Plugins
To see all plugins installed on your local machine:
```bash
pairee plugin list
```

### 🔄 Check for Updates
To check if any of your installed plugins have new versions in the registry:
```bash
pairee plugin check-updates
```

### 🆙 Update Plugins
To update all your non-pinned plugins to the latest version:
```bash
pairee plugin update
```

---

## 🛠️ Developing and Submitting Plugins

Want to add your own plugin to the registry? We encourage everyone to contribute!

1. **Read the Developer Guide**: Learn how to write Lua plugins and build custom interfaces by reading the [Plugin Developer Guide (English)](https://github.com/FittyAr/Pairee/blob/master/docs/plugin-dev-guide.md) or the [Guía de Desarrollo (Spanish)](https://github.com/FittyAr/Pairee/blob/master/docs/plugin-dev-guide-es.md).
2. **Review System Design**: For in-depth technical details on the plugin runtime, sandbox, and hooks system, see the [Plugin System Design Specification](https://github.com/FittyAr/Pairee/blob/master/docs/technical/plugin-system-design.md).
3. **Follow Submission Guidelines**: Read the [CONTRIBUTING.md](./CONTRIBUTING.md) guide in this branch to learn how to structure your plugin and submit a Pull Request to register your plugin.

---

## 📁 Registry Directory Structure

This branch uses a partitioned directory layout to scale efficiently:

```text
plugin-registry/
├── .gitignore
├── AGENTS.md                           # Instructions for agentic assistants
├── README.md                           # This file
├── CONTRIBUTING.md                     # Contributor guidelines
├── CODE_OF_CONDUCT.md                  # Code of Conduct
├── LICENSE                             # Legal terms
├── SECURITY.md                         # Security policies
├── SUPPORT.md                          # Support resources
└── registry/
    ├── index.toml                      # Master plugin catalog index
    ├── blocklist.toml                  # Emergency blocklist
    └── plugins/                        # Catalog root folder
        └── <author_initial>/           # Lowercase first letter of the author's name
            └── <author_name>/          # Author's handle/folder (case-preserved)
                └── <plugin_name>/      # Plugin source folder
                    ├── manifest.toml   # Manifest descriptor
                    ├── main.lua        # Entrypoint script
                    └── sha256.sum      # SHA-256 hashes of all plugin files
```

---

## 🛡️ Registry Management & Moderation

To ensure user safety, this registry is curated and monitored:
- **Integrity Checks**: All files inside a plugin release have their SHA-256 checksums calculated and verified.
- **Emergency Blocklist**: In case a security vulnerability or critical crash is discovered in a published plugin, maintainers can remotely disable or automatically remove it for all users by updating `registry/blocklist.toml`.
