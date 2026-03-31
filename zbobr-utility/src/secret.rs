/// A secret value that is either provided inline or resolved from an environment variable.
///
/// Deserializes from TOML as:
/// - `{ value = "secret" }` — inline literal value
/// - `{ env = "ENV_VAR" }` — name of an environment variable to read at runtime
///
/// # Usage
///
/// 1. Deserialize (or construct with [`Secret::value`] / [`Secret::env`]).
/// 2. Call [`Secret::resolve`] (e.g. in a `validate()` method) to fetch and cache the value.
/// 3. Use [`AsRef<str>`] to access the resolved string; panics if `resolve` was never called.
#[derive(Clone, Debug)]
pub struct Secret {
    source: SecretSource,
    resolved: Option<String>,
}

#[derive(Clone, Debug)]
enum SecretSource {
    Value(String),
    Env(String),
}

impl Secret {
    /// Create a `Secret` from an inline string value.
    pub fn value(s: impl Into<String>) -> Self {
        Self {
            source: SecretSource::Value(s.into()),
            resolved: None,
        }
    }

    /// Create a `Secret` from an environment variable name.
    pub fn env(var: impl Into<String>) -> Self {
        Self {
            source: SecretSource::Env(var.into()),
            resolved: None,
        }
    }

    /// Resolve and cache the secret string.
    ///
    /// For `Value`, caches the inline string directly.
    /// For `Env`, reads the named environment variable and caches its value.
    ///
    /// After a successful call, [`AsRef<str>`] returns the cached value.
    pub fn resolve(&mut self) -> anyhow::Result<&str> {
        if self.resolved.is_none() {
            let value = match &self.source {
                SecretSource::Value(v) => v.clone(),
                SecretSource::Env(var) => std::env::var(var)
                    .map_err(|_| anyhow::anyhow!("Environment variable '{}' is not set", var))?,
            };
            self.resolved = Some(value);
        }
        Ok(self.resolved.as_deref().unwrap())
    }

    /// Returns `true` if [`resolve`](Secret::resolve) has been successfully called.
    pub fn is_resolved(&self) -> bool {
        self.resolved.is_some()
    }
}

impl AsRef<str> for Secret {
    /// Returns the resolved secret string.
    ///
    /// # Panics
    ///
    /// Panics if [`Secret::resolve`] has not been called successfully yet.
    fn as_ref(&self) -> &str {
        self.resolved
            .as_deref()
            .expect("Secret::as_ref called before resolve(); call resolve() first")
    }
}

impl Default for Secret {
    fn default() -> Self {
        Self::value("")
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
            Helper::Value(f) => Ok(Secret::value(f.value)),
            Helper::Env(f) => Ok(Secret::env(f.env)),
        }
    }
}

impl serde::Serialize for Secret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        match &self.source {
            SecretSource::Value(v) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("value", v)?;
                map.end()
            }
            SecretSource::Env(var) => {
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
        assert!(!s.is_resolved());
        assert!(matches!(&s.source, SecretSource::Value(v) if v == "my-secret"));
    }

    #[test]
    fn deserialize_env_form() {
        let s: Secret = toml::from_str(r#"env = "MY_ENV_VAR""#).unwrap();
        assert!(matches!(&s.source, SecretSource::Env(v) if v == "MY_ENV_VAR"));
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
        let mut s = Secret::value("tok123");
        assert!(!s.is_resolved());
        assert_eq!(s.resolve().unwrap(), "tok123");
        assert!(s.is_resolved());
        assert_eq!(s.as_ref(), "tok123");
    }

    #[test]
    fn resolve_env_set() {
        unsafe {
            std::env::set_var("ZBOBR_TEST_SECRET_VAR", "env-value");
        }
        let mut s = Secret::env("ZBOBR_TEST_SECRET_VAR");
        assert_eq!(s.resolve().unwrap(), "env-value");
        assert_eq!(s.as_ref(), "env-value");
        unsafe {
            std::env::remove_var("ZBOBR_TEST_SECRET_VAR");
        }
    }

    #[test]
    fn resolve_env_missing() {
        unsafe {
            std::env::remove_var("ZBOBR_TEST_MISSING_VAR");
        }
        let mut s = Secret::env("ZBOBR_TEST_MISSING_VAR");
        assert!(s.resolve().is_err());
        assert!(!s.is_resolved());
    }

    #[test]
    fn as_ref_panics_if_not_resolved() {
        let s = Secret::value("tok");
        let result = std::panic::catch_unwind(|| {
            let _ = s.as_ref();
        });
        assert!(result.is_err());
    }

    #[test]
    fn as_ref_panics_for_value_variant_before_resolve() {
        // Even when the source is a plain Value, resolve() must be called first.
        let s = Secret::value("some-token");
        let result = std::panic::catch_unwind(|| s.as_ref().to_owned());
        assert!(result.is_err());
    }

    #[test]
    fn clone_preserves_resolved_state() {
        let mut s = Secret::value("tok");
        s.resolve().unwrap();
        let cloned = s.clone();
        assert!(cloned.is_resolved());
        assert_eq!(cloned.as_ref(), "tok");
    }

    #[test]
    fn clone_of_unresolved_is_unresolved() {
        let s = Secret::value("tok");
        let cloned = s.clone();
        assert!(!cloned.is_resolved());
    }

    #[test]
    fn serialize_value_form() {
        let s = Secret::value("my-secret");
        let serialized = toml::to_string(&s).unwrap();
        assert!(serialized.contains("value = \"my-secret\""));
    }

    #[test]
    fn serialize_env_form() {
        let s = Secret::env("MY_VAR");
        let serialized = toml::to_string(&s).unwrap();
        assert!(serialized.contains("env = \"MY_VAR\""));
    }

    #[test]
    fn resolve_caches_result() {
        let mut s = Secret::value("cached");
        s.resolve().unwrap();
        // Second call returns cached value without re-evaluating
        assert_eq!(s.resolve().unwrap(), "cached");
    }
}
