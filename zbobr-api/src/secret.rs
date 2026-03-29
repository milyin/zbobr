/// A secret value that is either provided inline or resolved from an environment variable.
///
/// Deserializes from TOML as:
/// - `{ value = "secret" }` — inline literal value
/// - `{ env = "ENV_VAR" }` — name of an environment variable to read at runtime
///
/// Use [`Secret::resolve`] to obtain the actual secret string.
#[derive(Clone, Debug)]
pub enum Secret {
    /// Inline literal secret value.
    Value(String),
    /// Name of an environment variable whose value is the secret.
    Env(String),
}

impl Secret {
    /// Resolve the secret to its string value.
    ///
    /// For `Value(s)`, returns `s` directly.
    /// For `Env(var)`, reads the environment variable `var` and returns its value,
    /// or returns an error if the variable is not set.
    pub fn resolve(&self) -> anyhow::Result<String> {
        match self {
            Secret::Value(v) => Ok(v.clone()),
            Secret::Env(var) => std::env::var(var)
                .map_err(|_| anyhow::anyhow!("Environment variable '{}' is not set", var)),
        }
    }
}

impl Default for Secret {
    fn default() -> Self {
        Secret::Value(String::new())
    }
}

impl<'de> serde::Deserialize<'de> for Secret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct ValueForm {
            value: String,
        }

        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct EnvForm {
            env: String,
        }

        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Value(ValueForm),
            Env(EnvForm),
        }

        match Helper::deserialize(deserializer)? {
            Helper::Value(f) => Ok(Secret::Value(f.value)),
            Helper::Env(f) => Ok(Secret::Env(f.env)),
        }
    }
}

impl serde::Serialize for Secret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        match self {
            Secret::Value(v) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("value", v)?;
                map.end()
            }
            Secret::Env(var) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("env", var)?;
                map.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_value_form() {
        let s: Secret = toml::from_str(r#"value = "my-secret""#).unwrap();
        assert!(matches!(s, Secret::Value(v) if v == "my-secret"));
    }

    #[test]
    fn deserialize_env_form() {
        let s: Secret = toml::from_str(r#"env = "MY_ENV_VAR""#).unwrap();
        assert!(matches!(s, Secret::Env(v) if v == "MY_ENV_VAR"));
    }

    #[test]
    fn deserialize_rejects_unknown_keys() {
        let result: Result<Secret, _> = toml::from_str(
            r#"value = "x"
extra = "y""#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn resolve_value() {
        let s = Secret::Value("tok123".to_string());
        assert_eq!(s.resolve().unwrap(), "tok123");
    }

    #[test]
    fn resolve_env_set() {
        unsafe {
            std::env::set_var("ZBOBR_TEST_SECRET_VAR", "env-value");
        }
        let s = Secret::Env("ZBOBR_TEST_SECRET_VAR".to_string());
        assert_eq!(s.resolve().unwrap(), "env-value");
        unsafe {
            std::env::remove_var("ZBOBR_TEST_SECRET_VAR");
        }
    }

    #[test]
    fn resolve_env_missing() {
        unsafe {
            std::env::remove_var("ZBOBR_TEST_MISSING_VAR");
        }
        let s = Secret::Env("ZBOBR_TEST_MISSING_VAR".to_string());
        assert!(s.resolve().is_err());
    }

    #[test]
    fn serialize_value_form() {
        let s = Secret::Value("my-secret".to_string());
        let serialized = toml::to_string(&s).unwrap();
        assert!(serialized.contains("value = \"my-secret\""));
    }

    #[test]
    fn serialize_env_form() {
        let s = Secret::Env("MY_VAR".to_string());
        let serialized = toml::to_string(&s).unwrap();
        assert!(serialized.contains("env = \"MY_VAR\""));
    }
}
