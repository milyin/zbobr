## Plan: Add tests for multi-config merge behavior

### Rationale
The multi-config feature is fully implemented and verified. These 3 test additions lock down the critical behavioral contract: `Option<Vec<T>>` deserialization preserves `None` vs `Some(vec![])` distinction, and merge semantics work correctly through the full TOML→struct→merge pipeline.

### Checklist items
1. **TOML deserialization round-trip tests** — verify serde preserves None/Some(vec![])/Some(vec![...]) for RoleDefinition.mcp and StageDefinition.prompts
2. **Tools map merge test** — verify IndexMap<String, Vec<ToolEntry>> merges key-wise with wholesale Vec replacement per key
3. **End-to-end merge from TOML strings** — parse two TOML snippets, merge, verify Vec field semantics compose correctly

### Key design decisions
- All tests go in the existing `#[cfg(test)]` module in `zbobr-api/src/config.rs` alongside the existing merge tests
- Follow the same test style as existing tests (lines 1788-2265)
- The end-to-end test uses `toml::from_str` to exercise the full deserialization+merge path

### Analog
Existing merge tests in `zbobr-api/src/config.rs` (role_partial_patch, role_mcp_cleared_by_empty_list, stage_prompts_cleared_by_empty_list, etc.)

### Verification
`cargo test -p zbobr-api --lib` — all tests including new ones should pass.