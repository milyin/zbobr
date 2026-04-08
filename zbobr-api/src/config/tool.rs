/// A tool name within workflow stage config.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tool(pub std::borrow::Cow<'static, str>);

impl Tool {
    pub const fn new(value: &'static str) -> Self {
        Tool(std::borrow::Cow::Borrowed(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::ops::Deref for Tool {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for Tool {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Tool {
    fn from(s: &str) -> Self {
        Tool(std::borrow::Cow::Owned(s.to_string()))
    }
}

impl From<String> for Tool {
    fn from(s: String) -> Self {
        Tool(std::borrow::Cow::Owned(s))
    }
}

impl From<Tool> for String {
    fn from(tool: Tool) -> Self {
        tool.0.into_owned()
    }
}

impl serde::Serialize for Tool {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for Tool {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Tool(std::borrow::Cow::Owned(String::deserialize(
            deserializer,
        )?)))
    }
}

impl schemars::JsonSchema for Tool {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Tool".into()
    }

    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string" })
    }
}
