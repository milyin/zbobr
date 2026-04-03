# Test: ToolEntry.priority serde round-trip

**File:** `zbobr-api/src/config.rs` (in the `#[cfg(test)]` module)

## Tests to add

### 1. `tool_entry_priority_deserializes_from_toml`
Parse a TOML `DispatcherConfigToml` snippet that includes `priority = 0` on a tool entry and assert:
- The resulting `ToolEntry.priority` is `Some(0)`

```toml
[dispatcher.tools]
developer = [
  { provider = "claude", model = "claude-opus-4.6", priority = 0 }
]
```

### 2. `tool_entry_priority_defaults_to_none`
Parse a TOML snippet with a tool entry that has **no** `priority` field and assert:
- The resulting `ToolEntry.priority` is `None`

### 3. `tool_entry_priority_none_skipped_in_serialization`
Serialize a `ToolEntry { priority: None, ... }` to TOML (via `toml::to_string`) and assert:
- The output does **not** contain the word `priority`

### 4. `tool_entry_priority_some_included_in_serialization`
Serialize a `ToolEntry { priority: Some(5), ... }` to TOML and assert:
- The output contains `priority = 5`

## Rationale
The `#[serde(default, skip_serializing_if = "Option::is_none")]` attributes need to be verified, as other config fields (e.g. `ProviderDefinition.priority`) have analogous tests. The absence of tests means a typo in the attribute would go undetected.
