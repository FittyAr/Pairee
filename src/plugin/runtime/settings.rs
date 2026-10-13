//! `pairee.settings`: the plugin's `[settings_schema]` defaults merged with
//! the user's values, which the host passes in (from the `config.toml` it
//! already loaded).
//!
//! The user's values are stored as strings; each one is converted with the
//! type the schema declares (`type = "bool" | "integer" | "number" |
//! "string"`, or the type of `default` when `type` is missing). A value
//! that does not parse as that type is ignored (with a warning) and the
//! default is used instead.

use crate::plugin::loader::PluginManifest;
use mlua::{Lua, Table, Value};
use std::collections::HashMap;

/// The type a setting declares in the schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Bool,
    Integer,
    Number,
    Text,
}

impl Kind {
    fn of(entry: &toml::Table) -> Self {
        match entry.get("type").and_then(toml::Value::as_str) {
            Some("bool" | "boolean") => Self::Bool,
            Some("integer" | "int") => Self::Integer,
            Some("number" | "float") => Self::Number,
            Some(_) => Self::Text,
            None => match entry.get("default") {
                Some(toml::Value::Boolean(_)) => Self::Bool,
                Some(toml::Value::Integer(_)) => Self::Integer,
                Some(toml::Value::Float(_)) => Self::Number,
                _ => Self::Text,
            },
        }
    }

    fn parse<'lua>(self, lua: &'lua Lua, raw: &str) -> mlua::Result<Option<Value<'lua>>> {
        let raw = raw.trim();
        Ok(match self {
            Self::Bool => raw.parse().ok().map(Value::Boolean),
            Self::Integer => raw.parse().ok().map(Value::Integer),
            Self::Number => raw.parse().ok().map(Value::Number),
            Self::Text => Some(Value::String(lua.create_string(raw)?)),
        })
    }
}

fn toml_to_lua<'lua>(lua: &'lua Lua, value: &toml::Value) -> mlua::Result<Value<'lua>> {
    Ok(match value {
        toml::Value::Boolean(b) => Value::Boolean(*b),
        toml::Value::Integer(i) => Value::Integer(*i),
        toml::Value::Float(f) => Value::Number(*f),
        toml::Value::String(s) => Value::String(lua.create_string(s)?),
        _ => Value::Nil,
    })
}

/// Builds the `pairee.settings` table.
pub fn build<'lua>(
    lua: &'lua Lua,
    manifest: Option<&PluginManifest>,
    user: &HashMap<String, String>,
) -> mlua::Result<Table<'lua>> {
    let table = lua.create_table()?;
    let Some(schema) = manifest.and_then(|m| m.settings_schema.as_ref()) else {
        return Ok(table);
    };
    for (key, entry) in schema {
        let Some(entry) = entry.as_table() else {
            continue;
        };
        let kind = Kind::of(entry);
        let user_value = match user.get(key) {
            Some(raw) => {
                let parsed = kind.parse(lua, raw)?;
                if parsed.is_none() {
                    tracing::warn!(
                        "Plugin setting {key:?} = {raw:?} is not a valid {kind:?}; using the default"
                    );
                }
                parsed
            }
            None => None,
        };
        let value = match user_value {
            Some(v) => v,
            None => match entry.get("default") {
                Some(default) => toml_to_lua(lua, default)?,
                None => Value::Nil,
            },
        };
        table.set(key.as_str(), value)?;
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> PluginManifest {
        PluginManifest::parse(
            r#"
            name = "demo"
            version = "1.0.0"
            [settings_schema]
            flag = { type = "bool", default = false }
            depth = { type = "integer", default = 3 }
            ratio = { type = "number", default = 0.5 }
            label = { type = "string", default = "x" }
            code = { type = "string", default = "007" }
            inferred = { default = 10 }
            "#,
        )
        .unwrap()
    }

    fn user(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn defaults_apply_without_user_values() {
        let lua = Lua::new();
        let t = build(&lua, Some(&manifest()), &HashMap::new()).unwrap();
        assert!(!t.get::<_, bool>("flag").unwrap());
        assert_eq!(t.get::<_, i64>("depth").unwrap(), 3);
        assert_eq!(t.get::<_, f64>("ratio").unwrap(), 0.5);
        assert_eq!(t.get::<_, String>("label").unwrap(), "x");
    }

    #[test]
    fn user_values_follow_the_schema_type() {
        let lua = Lua::new();
        let values = user(&[
            ("flag", "true"),
            ("depth", "7"),
            ("ratio", "1.5"),
            ("code", "123"),
            ("inferred", "11"),
        ]);
        let t = build(&lua, Some(&manifest()), &values).unwrap();
        assert!(t.get::<_, bool>("flag").unwrap());
        assert_eq!(t.get::<_, i64>("depth").unwrap(), 7);
        assert_eq!(t.get::<_, f64>("ratio").unwrap(), 1.5);
        // A string setting stays a string even when it looks like a number.
        assert!(matches!(
            t.get::<_, Value>("code").unwrap(),
            Value::String(_)
        ));
        assert!(matches!(
            t.get::<_, Value>("inferred").unwrap(),
            Value::Integer(11)
        ));
    }

    #[test]
    fn invalid_user_values_fall_back_to_the_default() {
        let lua = Lua::new();
        let t = build(&lua, Some(&manifest()), &user(&[("depth", "deep")])).unwrap();
        assert_eq!(t.get::<_, i64>("depth").unwrap(), 3);
    }
}
