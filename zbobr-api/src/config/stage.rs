/// A stage name within a pipeline (user-defined, dynamically configured).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Stage(pub std::borrow::Cow<'static, str>);

impl Stage {
    pub const fn new_static(s: &'static str) -> Self {
        Stage(std::borrow::Cow::Borrowed(s))
    }

    pub fn new(s: impl Into<String>) -> Self {
        Stage(std::borrow::Cow::Owned(s.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::ops::Deref for Stage {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for Stage {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Stage {
    fn from(s: &str) -> Self {
        Stage(std::borrow::Cow::Owned(s.to_string()))
    }
}

impl From<String> for Stage {
    fn from(s: String) -> Self {
        Stage(std::borrow::Cow::Owned(s))
    }
}

impl serde::Serialize for Stage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for Stage {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Stage(std::borrow::Cow::Owned(String::deserialize(deserializer)?)))
    }
}

impl schemars::JsonSchema for Stage {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Stage".into()
    }

    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string" })
    }
}
