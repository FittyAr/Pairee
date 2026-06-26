---
name: localize-helper
description: Guide AI agents in adding or modifying localized strings within the Pairee project.
---

# Localize Helper Skill

Use this skill when you need to add, modify, or translate any user-facing text in the Pairee project, adhering to the "Zero Hardcoding" rule.

## Guidelines

1. **Never Hardcode User-Facing Strings**: All UI text must be fetched dynamically using the translation helper `crate::config::localization::t("key")`.
2. **English Translation Source**: Centralize all English defaults inside `get_default_english_translation` within `src/config/localization/en.rs`.
3. **Spanish Translation Source**: Store Spanish localizations inside the JSON file `lang/es.json`.

## Procedure

### Step 1: Add/Update English Key
Open `src/config/localization/en.rs` and add your key-value pair under the appropriate category:

```rust
"my_new_key" => "My localized text",
```

### Step 2: Add/Update Spanish Key
Open `lang/es.json` and insert the matching key-value pair:

```json
"my_new_key": "Mi texto traducido",
```

### Step 3: Implement in Code
Utilize the key in the code. Import `t` from `crate::config::localization::t` and use it:

```rust
use crate::config::localization::t;

// Inside UI rendering or panel drawing:
let label = t("my_new_key");
```

### Step 4: Verification
Verify that both files are properly formatted:
- Run `cargo fmt` to check Rust files.
- Ensure the JSON file `lang/es.json` is valid JSON and alphabetically sorted or matches existing structure.
