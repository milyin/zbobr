All three checklist items are complete. I added 8 tests covering:

1. **Serde round-trip tests** (6 tests) — verify `None` vs `Some(vec![])` distinction is preserved through TOML deserialization for both `RoleDefinition.mcp` and `StageDefinition.prompts`
2. **Tools map merge test** (1 test) — verifies `IndexMap<String, Vec<ToolEntry>>` key-wise merge with wholesale list replacement per key
3. **End-to-end merge test** (1 test) — parses two TOML strings, merges them, and verifies all merge behaviors compose correctly (override, inherit, clear)

All 126 zbobr-api tests pass.