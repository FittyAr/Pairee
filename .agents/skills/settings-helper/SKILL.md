---
name: settings-helper
description: Guide AI agents in adding or modifying user settings in the Pairee config module.
---

# Settings Helper Skill

Use this skill when you need to introduce new settings, update default configurations, or expose settings in the UI.

## Structure of Settings

Pairee manages configuration settings inside `src/config/settings.rs`. The settings structure is serialized/deserialized using TOML.

### Step 1: Add Fields to `Settings` Struct
Locate `Settings` in `src/config/settings.rs` and add the new configuration fields. Ensure proper serde annotations (e.g. `#[serde(default)]` or `#[serde(skip_serializing_if = "Option::is_none")]`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // ... existing fields ...
    #[serde(default)]
    pub my_new_setting: bool,
}
```

### Step 2: Define Default Values
Provide defaults in the `Default` implementation block for `Settings`:

```rust
impl Default for Settings {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            my_new_setting: false,
        }
    }
}
```

### Step 3: Implement Setting Exposure in the UI
If this setting is user-configurable from the UI, register its control logic within the settings popup or input forms. Follow existing patterns for checkbox toggles, input strings, or dropdown selects.
