//! Stdio kinds for `pairee.Command` (`NULL` / `PIPED` / `INHERIT`).

use mlua::Value;
use std::process::Stdio;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdioKind {
    Null,
    Piped,
    Inherit,
}

impl StdioKind {
    pub const NULL: u8 = 0;
    pub const PIPED: u8 = 1;
    pub const INHERIT: u8 = 2;

    pub fn to_std(self) -> Stdio {
        match self {
            Self::Null => Stdio::null(),
            Self::Piped => Stdio::piped(),
            Self::Inherit => Stdio::inherit(),
        }
    }

    pub fn from_lua(value: &Value) -> Self {
        match value {
            Value::Integer(0) => Self::Null,
            Value::Integer(2) => Self::Inherit,
            Value::Integer(_) => Self::Piped,
            Value::String(s) => match s.to_str().map(|cow| cow.to_ascii_lowercase()) {
                Ok(ref k) if k == "null" => Self::Null,
                Ok(ref k) if k == "inherit" => Self::Inherit,
                _ => Self::Piped,
            },
            _ => Self::Piped,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn parses_integer_and_string_tags() {
        let lua = Lua::new();
        assert_eq!(StdioKind::from_lua(&Value::Integer(0)), StdioKind::Null);
        assert_eq!(StdioKind::from_lua(&Value::Integer(1)), StdioKind::Piped);
        assert_eq!(StdioKind::from_lua(&Value::Integer(2)), StdioKind::Inherit);
        let s = lua.create_string("null").unwrap();
        assert_eq!(StdioKind::from_lua(&Value::String(s)), StdioKind::Null);
    }
}
