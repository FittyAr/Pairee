# Contributing to Pairee Plugin Registry

Thank you for contributing your plugin to the Pairee ecosystem! This document guides you through the process of submitting new plugins or updating existing ones.

---

## 🛠️ Automated Submission (Recommended)

Pairee includes developer tools built right into the core CLI. The easiest way to package, validate, and submit your plugin is using the automated wizard:

1. Open your terminal in your local plugin directory.
2. Run the submission wizard:
   ```bash
   pairee developer submit
   ```
3. Follow the interactive prompts:
   - Provide a commit description.
   - Enter your GitHub Personal Access Token (PAT) with `public_repo` scope.
4. The tool will automatically:
   - Validate your plugin files, icon, and screenshots.
   - Fork this repository to your GitHub account.
   - Package the files into the correct registry layout.
   - Update `registry/index.toml`.
   - Push to your fork and submit a Pull Request to this branch!

---

## ✍️ Manual Submission Workflow

If you prefer to submit manually, follow these steps:

### 1. Fork and Clone
Fork this repository and clone the **`plugin-registry`** branch:
```bash
git clone -b plugin-registry https://github.com/YOUR_USERNAME/Pairee.git
cd Pairee
```

### 2. Add or Update Your Plugin Folder
Plugins must be placed in the partitioned directory:
`registry/plugins/<author_initial_lowercase>/<author_name>/<plugin_name>/`

**Example Layout**:
For author `FittyAr` and plugin `test-plugin`:
```text
registry/plugins/f/FittyAr/test-plugin/
├── manifest.toml       # Manifest file (with valid name, version, author, etc.)
├── main.lua           # Main Lua script
├── sha256.sum         # File containing SHA-256 checksums of all plugin files
├── help/              # Help docs
│   └── en.md
└── lang/              # Localized translation files
    └── en.toml
```

### 3. Generate SHA-256 Checksums (`sha256.sum`)
Every file in your plugin folder must be listed in a `sha256.sum` file inside your plugin's directory. Format:
```text
<sha256_hash>  <relative_file_path>
```
*Note: You can generate this automatically by running `pairee developer package` inside your plugin folder.*

### 4. Update the Master Registry Index (`registry/index.toml`) & Manifest
Append or update your plugin details in the master `registry/index.toml` (note that `index.toml` only holds plugin metadata; the actual files and their SHA-256 hashes must be appended to the copied `manifest.toml` under a `[files]` table):

**registry/index.toml entry**:
```toml
[plugins.<plugin_name>]
name = "<plugin_name>"
version = "<plugin_version>"
description = "<plugin_description>"
author = "<author_name>"
languages = ["en", "es"]
hooks = []
min_pairee = "0.6.1"
```

**registry/plugins/.../<plugin_name>/manifest.toml appended files table**:
```toml
[files]
"main.lua" = "<sha256_hash>"
"manifest.toml" = "<sha256_hash>"
"sha256.sum" = "<sha256_hash>"
"LICENSE" = "<sha256_hash>"
```

### 5. License Requirements
All plugins must be licensed.
- If your plugin workspace does not contain a license file, `pairee developer package` will automatically assign the `"MIT"` license in the manifest and generate a standard `LICENSE` file for you.
- If a license file is present but the manifest `license` field is empty, the tool will prompt you for the license name.

### 6. Commit and Create a Pull Request
Commit your changes, push to your fork, and open a Pull Request (PR) targeting the `plugin-registry` branch.
```bash
git add .
git commit -m "Add/Update plugin <plugin_name> v<version>"
git push origin plugin-registry
```

---

## 🚫 Blocklist and Moderation Policy

To maintain a secure and reliable ecosystem, registry maintainers reserves the right to:
- Review and request changes on submissions.
- Block or disable plugins that crash the application, contain malware, or violate our [Code of Conduct](./CODE_OF_CONDUCT.md).
- Blocked plugins are added to `registry/blocklist.toml` and are automatically hidden from search/listing and removed from user setups upon update.
