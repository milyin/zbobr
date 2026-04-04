## Verification of Vec-under-Option assumption

### All confirmed correct:
- All Vec fields in TOML structs are `Option<Vec<T>>` — no bare Vec in mergeable structs
- All `*Patch` types fully removed, no dangling references
- Merge semantics: `None` = inherit, `Some(vec![])` = explicitly empty (via `.or()`)
- Runtime semantics: `None` and `Some(vec![])` treated identically everywhere (no items)
- Accessors in `prompts.rs` and `cli.rs` handle Option correctly
- Build succeeds, all relevant tests pass (3 failures in `zbobr-task-backend-github` are pre-existing TLS setup issues)

### Intentional behavioral change noted:
- Old: role with absent `mcp` → all MCP tools allowed
- New: role with absent `mcp` → no MCP tools (test fixtures updated accordingly)

### Proposed test additions to lock down behavior:
1. **TOML deserialization round-trip for Option\<Vec\>**: Verify missing field → None, empty list → Some(vec![]), populated list → Some(vec![...]) for both RoleDefinition and StageDefinition
2. **Tools map merge**: Test that `IndexMap<String, Vec<ToolEntry>>` merges key-wise with wholesale Vec replacement for same-key entries
3. **End-to-end multi-config merge from TOML strings**: Parse two TOML config snippets, merge them, verify Vec fields behave correctly (None inherits, Some overrides)