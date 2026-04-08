/// A provider identifier within dispatcher config and tool definitions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Provider(pub std::borrow::Cow<'static, str>);

impl Provider {
    pub const fn new(value: &'static str) -> Self {
        Provider(std::borrow::Cow::Borrowed(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::ops::Deref for Provider {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for Provider {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Provider {
    fn from(s: &str) -> Self {
        Provider(std::borrow::Cow::Owned(s.to_string()))
    }
}

impl From<String> for Provider {
    fn from(s: String) -> Self {
        Provider(std::borrow::Cow::Owned(s))
    }
}

impl From<Provider> for String {
    fn from(provider: Provider) -> Self {
        provider.0.into_owned()
    }
}

impl serde::Serialize for Provider {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for Provider {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Provider(std::borrow::Cow::Owned(String::deserialize(
            deserializer,
        )?)))
    }
}

impl schemars::JsonSchema for Provider {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Provider".into()
    }

    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string" })
    }
}
