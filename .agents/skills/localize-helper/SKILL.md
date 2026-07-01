---
name: localize-helper
description: Guide AI agents in adding or modifying localized strings within the Pairee project.
---

# Localize Helper Skill

Use this skill when you need to add, modify, or translate any user-facing text in the Pairee project, adhering to the "Zero Hardcoding" rule.

## Guidelines

1. **Never Hardcode User-Facing Strings**: All UI text must be fetched dynamically using the translation helper `crate::config::localization::t("key")`.
2. **English Translation Source**: Store all English defaults inside `lang/en.toml` under the `[translations]` section.
3. **Spanish Translation Source**: Store Spanish localizations inside `lang/es.toml` under the `[translations]` section.

## Procedure

### Step 1: Add/Update English Key
Open `lang/en.toml` and add your key-value pair alphabetically under `[translations]`:

```toml
my_new_key = "My localized text"
```

### Step 2: Add/Update Spanish Key
Open `lang/es.toml` and insert the matching key-value pair alphabetically under `[translations]`:

```toml
my_new_key = "Mi texto traducido"
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
- Run `python scripts/clean_translations.py` to ensure there are no unused keys remaining.
