In zbobr-api/src/config.rs:
- Change `priority: i32` (with `#[serde(default = "default_provider_priority")]`) to `priority: Option<i32>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`
- Remove the `default_provider_priority()` helper function
- In `resolve_single_provider()`: for child providers use `def.priority.unwrap_or(parent.priority)`, for root providers use `def.priority.unwrap_or(10)`
- Update all test ProviderDefinition constructors: change `priority: 10` to `priority: None`, and `priority: 5` to `priority: Some(5)`