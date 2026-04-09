/// A three-state configuration value for layered TOML merging.
///
/// In the zbobr config system, `Option<T>` conflates "absent" with "explicitly cleared".
/// `TomlOption<T>` distinguishes three states:
///
/// - **`Absent`** — field not present in TOML; inherit from base during merge.
/// - **`ExplicitNone`** — field set to `nan` in TOML; clears any base value.
/// - **`Value(T)`** — normal value; overrides base.
///
/// # Serde behaviour
///
/// - A missing TOML key deserializes as `Absent` (requires `#[serde(default)]`).
/// - The TOML literal `nan` deserializes as `ExplicitNone`.
/// - Any other value deserializes as `Value(T)` via `T::Deserialize`.
///
/// Serialization: `Absent` is skipped (use `#[serde(skip_serializing_if = "TomlOption::is_absent")]`),
/// `ExplicitNone` serializes as `f64::NAN`, `Value(T)` serializes `T` normally.
#[derive(Clone, Debug, PartialEq)]
pub enum TomlOption<T> {
    /// Field was not present in the TOML source.
    Absent,
    /// Field was explicitly set to `nan` — clears inherited value.
    ExplicitNone,
    /// Field carries a normal value.
    Value(T),
}

impl<T> Default for TomlOption<T> {
    fn default() -> Self {
        Self::Absent
    }
}

impl<T> TomlOption<T> {
    /// Returns `true` if this is `Absent`.
    pub fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    /// Returns `true` if this is `Value(_)`.
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Value(_))
    }

    /// Returns `true` if this is `Absent` or `ExplicitNone`.
    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    /// Convert to `Option<&T>`: `Value(t)` → `Some(&t)`, otherwise `None`.
    pub fn as_option(&self) -> Option<&T> {
        match self {
            Self::Value(v) => Some(v),
            _ => None,
        }
    }

    /// Convert to `Option<&mut T>`: `Value(t)` → `Some(&mut t)`, otherwise `None`.
    pub fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Value(v) => Some(v),
            _ => None,
        }
    }

    /// Convert to `Option<T>`: `Value(t)` → `Some(t)`, otherwise `None`.
    pub fn into_option(self) -> Option<T> {
        match self {
            Self::Value(v) => Some(v),
            _ => None,
        }
    }

    /// Map the inner value, preserving `Absent` and `ExplicitNone`.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> TomlOption<U> {
        match self {
            Self::Absent => TomlOption::Absent,
            Self::ExplicitNone => TomlOption::ExplicitNone,
            Self::Value(v) => TomlOption::Value(f(v)),
        }
    }

    /// Merge two `TomlOption` values: overlay wins unless it is `Absent`.
    ///
    /// Truth table (self=base, other=overlay → result):
    /// - `(_, Value(v))` → `Value(v)`
    /// - `(_, ExplicitNone)` → `ExplicitNone`
    /// - `(base, Absent)` → `base`
    pub fn merge(self, other: Self) -> Self {
        match other {
            Self::Absent => self,
            _ => other,
        }
    }
}

impl<T> From<Option<T>> for TomlOption<T> {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => Self::Value(v),
            None => Self::Absent,
        }
    }
}

// ---------------------------------------------------------------------------
// Serde
// ---------------------------------------------------------------------------

impl<T: serde::Serialize> serde::Serialize for TomlOption<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Absent => serializer.serialize_none(),
            Self::ExplicitNone => serializer.serialize_f64(f64::NAN),
            Self::Value(v) => v.serialize(serializer),
        }
    }
}

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for TomlOption<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(TomlOptionVisitor(std::marker::PhantomData))
    }
}

/// A serde visitor that intercepts NaN floats and delegates everything else to `T`.
struct TomlOptionVisitor<T>(std::marker::PhantomData<T>);

macro_rules! forward_visit {
    ($method:ident, $ty:ty) => {
        fn $method<E: serde::de::Error>(self, v: $ty) -> Result<Self::Value, E> {
            use serde::de::IntoDeserializer;
            T::deserialize(v.into_deserializer()).map(TomlOption::Value)
        }
    };
}

impl<'de, T: serde::Deserialize<'de>> serde::de::Visitor<'de> for TomlOptionVisitor<T> {
    type Value = TomlOption<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a value, nan, or absent field")
    }

    fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
        if v.is_nan() {
            Ok(TomlOption::ExplicitNone)
        } else {
            use serde::de::IntoDeserializer;
            T::deserialize(v.into_deserializer()).map(TomlOption::Value)
        }
    }

    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
        use serde::de::IntoDeserializer;
        T::deserialize(v.into_deserializer()).map(TomlOption::Value)
    }

    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
        use serde::de::IntoDeserializer;
        T::deserialize(v.into_deserializer()).map(TomlOption::Value)
    }

    forward_visit!(visit_bool, bool);
    forward_visit!(visit_str, &str);
    forward_visit!(visit_string, String);

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        T::deserialize(serde::de::value::SeqAccessDeserializer::new(seq)).map(TomlOption::Value)
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        T::deserialize(serde::de::value::MapAccessDeserializer::new(map)).map(TomlOption::Value)
    }

    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(TomlOption::Absent)
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(TomlOption::Absent)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

// ---------------------------------------------------------------------------
// MergeToml
// ---------------------------------------------------------------------------

impl<T: super::MergeToml> super::MergeToml for TomlOption<T> {
    fn merge_toml(self, other: Self) -> Self {
        match (self, other) {
            (TomlOption::Value(base), TomlOption::Value(over)) => {
                TomlOption::Value(base.merge_toml(over))
            }
            (base, TomlOption::Absent) => base,
            (_, over) => over,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Serde roundtrip tests ------------------------------------------------

    #[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize, Default)]
    #[serde(default)]
    struct Sample {
        #[serde(default, skip_serializing_if = "TomlOption::is_absent")]
        name: TomlOption<String>,
        #[serde(default, skip_serializing_if = "TomlOption::is_absent")]
        count: TomlOption<u32>,
        #[serde(default, skip_serializing_if = "TomlOption::is_absent")]
        flag: TomlOption<bool>,
    }

    #[test]
    fn deserialize_value() {
        let s: Sample = toml::from_str(r#"name = "hello""#).unwrap();
        assert_eq!(s.name, TomlOption::Value("hello".to_string()));
    }

    #[test]
    fn deserialize_nan_produces_explicit_none() {
        let s: Sample = toml::from_str("count = nan").unwrap();
        assert_eq!(s.count, TomlOption::ExplicitNone);
    }

    #[test]
    fn deserialize_absent_produces_absent() {
        let s: Sample = toml::from_str("").unwrap();
        assert_eq!(s.name, TomlOption::Absent);
        assert_eq!(s.count, TomlOption::Absent);
    }

    #[test]
    fn deserialize_normal_integer() {
        let s: Sample = toml::from_str("count = 42").unwrap();
        assert_eq!(s.count, TomlOption::Value(42));
    }

    #[test]
    fn deserialize_bool_value() {
        let s: Sample = toml::from_str("flag = true").unwrap();
        assert_eq!(s.flag, TomlOption::Value(true));
    }

    #[test]
    fn serialize_value() {
        let s = Sample {
            name: TomlOption::Value("world".to_string()),
            count: TomlOption::Absent,
            flag: TomlOption::Absent,
        };
        let serialized = toml::to_string(&s).unwrap();
        assert!(serialized.contains("name = \"world\""));
        assert!(!serialized.contains("count"));
        assert!(!serialized.contains("flag"));
    }

    #[test]
    fn serialize_explicit_none_produces_nan() {
        let s = Sample {
            name: TomlOption::Absent,
            count: TomlOption::ExplicitNone,
            flag: TomlOption::Absent,
        };
        let serialized = toml::to_string(&s).unwrap();
        assert!(serialized.contains("count = nan"));
    }

    // -- Merge truth table (9 combinations) -----------------------------------

    #[test]
    fn merge_value_over_value() {
        let base = TomlOption::Value(1);
        let overlay = TomlOption::Value(2);
        assert_eq!(base.merge(overlay), TomlOption::Value(2));
    }

    #[test]
    fn merge_explicit_none_over_value() {
        let base = TomlOption::Value(1);
        let overlay = TomlOption::ExplicitNone;
        assert_eq!(base.merge(overlay), TomlOption::ExplicitNone);
    }

    #[test]
    fn merge_absent_over_value() {
        let base = TomlOption::Value(1);
        let overlay = TomlOption::Absent;
        assert_eq!(base.merge(overlay), TomlOption::Value(1));
    }

    #[test]
    fn merge_value_over_explicit_none() {
        let base: TomlOption<i32> = TomlOption::ExplicitNone;
        let overlay = TomlOption::Value(3);
        assert_eq!(base.merge(overlay), TomlOption::Value(3));
    }

    #[test]
    fn merge_explicit_none_over_explicit_none() {
        let base: TomlOption<i32> = TomlOption::ExplicitNone;
        let overlay = TomlOption::ExplicitNone;
        assert_eq!(base.merge(overlay), TomlOption::ExplicitNone);
    }

    #[test]
    fn merge_absent_over_explicit_none() {
        let base: TomlOption<i32> = TomlOption::ExplicitNone;
        let overlay = TomlOption::Absent;
        assert_eq!(base.merge(overlay), TomlOption::ExplicitNone);
    }

    #[test]
    fn merge_value_over_absent() {
        let base: TomlOption<i32> = TomlOption::Absent;
        let overlay = TomlOption::Value(5);
        assert_eq!(base.merge(overlay), TomlOption::Value(5));
    }

    #[test]
    fn merge_explicit_none_over_absent() {
        let base: TomlOption<i32> = TomlOption::Absent;
        let overlay = TomlOption::ExplicitNone;
        assert_eq!(base.merge(overlay), TomlOption::ExplicitNone);
    }

    #[test]
    fn merge_absent_over_absent() {
        let base: TomlOption<i32> = TomlOption::Absent;
        let overlay = TomlOption::Absent;
        assert_eq!(base.merge(overlay), TomlOption::Absent);
    }

    // -- into_option / as_option conversions ----------------------------------

    #[test]
    fn into_option_value() {
        assert_eq!(TomlOption::Value(42).into_option(), Some(42));
    }

    #[test]
    fn into_option_explicit_none() {
        assert_eq!(TomlOption::<i32>::ExplicitNone.into_option(), None);
    }

    #[test]
    fn into_option_absent() {
        assert_eq!(TomlOption::<i32>::Absent.into_option(), None);
    }

    #[test]
    fn as_option_value() {
        let v = TomlOption::Value(42);
        assert_eq!(v.as_option(), Some(&42));
    }

    #[test]
    fn as_option_none_variants() {
        let v: TomlOption<i32> = TomlOption::ExplicitNone;
        assert_eq!(v.as_option(), None);
        let v: TomlOption<i32> = TomlOption::Absent;
        assert_eq!(v.as_option(), None);
    }

    // -- map ------------------------------------------------------------------

    #[test]
    fn map_value() {
        let v = TomlOption::Value(3);
        assert_eq!(v.map(|x| x * 2), TomlOption::Value(6));
    }

    #[test]
    fn map_preserves_absent() {
        let v: TomlOption<i32> = TomlOption::Absent;
        assert_eq!(v.map(|x| x * 2), TomlOption::Absent);
    }

    #[test]
    fn map_preserves_explicit_none() {
        let v: TomlOption<i32> = TomlOption::ExplicitNone;
        assert_eq!(v.map(|x| x * 2), TomlOption::ExplicitNone);
    }

    // -- Default / From -------------------------------------------------------

    #[test]
    fn default_is_absent() {
        assert_eq!(TomlOption::<String>::default(), TomlOption::Absent);
    }

    #[test]
    fn from_some() {
        let v: TomlOption<i32> = Some(10).into();
        assert_eq!(v, TomlOption::Value(10));
    }

    #[test]
    fn from_none() {
        let v: TomlOption<i32> = None.into();
        assert_eq!(v, TomlOption::Absent);
    }

    // -- MergeToml trait ------------------------------------------------------

    #[test]
    fn merge_toml_trait() {
        use crate::MergeToml;
        let base = TomlOption::Value(1i32);
        let overlay = TomlOption::ExplicitNone;
        assert_eq!(base.merge_toml(overlay), TomlOption::ExplicitNone);
    }
}
