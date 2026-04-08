/// A role name within workflow stages.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Role(pub std::borrow::Cow<'static, str>);

impl Role {
    pub const fn new(value: &'static str) -> Self {
        Role(std::borrow::Cow::Borrowed(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::ops::Deref for Role {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for Role {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Role {
    fn from(s: &str) -> Self {
        Role(std::borrow::Cow::Owned(s.to_string()))
    }
}

impl From<String> for Role {
    fn from(s: String) -> Self {
        Role(std::borrow::Cow::Owned(s))
    }
}

impl From<Role> for String {
    fn from(role: Role) -> Self {
        role.0.into_owned()
    }
}

impl serde::Serialize for Role {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for Role {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Role(std::borrow::Cow::Owned(String::deserialize(
            deserializer,
        )?)))
    }
}

impl schemars::JsonSchema for Role {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Role".into()
    }

    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string" })
    }
}
