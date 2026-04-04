Add tests to the existing `#[cfg(test)]` module in `zbobr-api/src/config.rs` that verify serde deserialization of `Option<Vec<T>>` fields.

**What to test:**
- Missing field in TOML → deserializes as `None`
- Empty list in TOML (e.g. `mcp = []`) → deserializes as `Some(vec![])`
- Populated list in TOML → deserializes as `Some(vec![...])`

**Which structs to test:** `RoleDefinition` (has `mcp: Option<Vec<McpTool>>`) and `StageDefinition` (has `prompts: Option<Vec<PathBuf>>`). These are the two structs with `Option<Vec<T>>` fields relevant to the merge feature.

**Why:** This locks down the serde contract that the merge logic depends on — `None` vs `Some(vec![])` distinction must be preserved through deserialization for the `.or()` merge semantics to work correctly.

**Pattern to follow:** The existing `tool_entry_deser_*` tests (around line 1540-1600) show the style for TOML deserialization tests — use inline TOML strings with `toml::from_str`.