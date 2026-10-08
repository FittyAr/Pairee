use std::collections::BTreeMap;

pub fn load_user_menu_commands() -> BTreeMap<String, String> {
    let path = crate::config::paths::get_config_dir().join("usermenu.toml");
    let mut commands = BTreeMap::new();
    if let Ok(content) = std::fs::read_to_string(&path)
        && let Ok(toml_val) = toml::from_str::<toml::Value>(&content)
        && let Some(cmds) = toml_val.get("commands").and_then(|v| v.as_table())
    {
        for (k, v) in cmds {
            if let Some(cmd_str) = v.as_str() {
                commands.insert(k.clone(), cmd_str.to_string());
            }
        }
    }
    commands
}
