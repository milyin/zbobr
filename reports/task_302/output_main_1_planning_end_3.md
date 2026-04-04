Plan is ready with 3 checklist items for the test additions:

1. **TOML deserialization round-trip tests** — verify `None`/`Some(vec![])`/`Some(vec![...])` preservation through serde for `RoleDefinition.mcp` and `StageDefinition.prompts`
2. **Tools map merge test** — verify `IndexMap<String, Vec<ToolEntry>>` key-wise merge with wholesale Vec replacement
3. **End-to-end merge from TOML strings** — parse two TOML snippets, merge, verify the full deserialization+merge pipeline

All tests will be added to the existing test module in `zbobr-api/src/config.rs`, following the established patterns.