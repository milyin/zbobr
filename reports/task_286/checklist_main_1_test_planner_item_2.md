# Test: default config uses "developer" tool

**File:** `zbobr/src/init.rs` (in the `#[cfg(test)]` module)

## Test to add

### `default_config_roles_reference_developer_tool`
1. Call `default_config_toml()` to get `RootConfigToml`.
2. Build a `ZbobrConfig` from it (or call `validate_config` if that path is accessible).
3. Assert that the `dispatcher.tools` map contains a key `"developer"`.
4. Assert that each role definition that has a `tool` field set to `"developer"` refers to an existing tool name.

Alternatively, if building a full `ZbobrConfig` is expensive, at minimum assert directly on `RootConfigToml`:
- `dispatcher_toml.tools` contains key `"developer"`
- No role's `tool` field references `"smart"` (the old name)

## Rationale
The rename from `"smart"` to `"developer"` in the default config affects all four role definitions. A regression (e.g., a partial rename) would produce an invalid config that only fails at runtime. This test provides a compile-time-safe regression guard.
