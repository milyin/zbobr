The review found a blocking regression in `zbobr-dispatcher/src/prompts.rs`. When a role has no explicit `mcp` list (`None`), the code now falls back to an empty slice `&[]` instead of all MCP tools (`ALL_TOOLS`). This breaks existing configs that rely on the implicit "all tools" default.

Fix: Change `.unwrap_or(&[])` to `.unwrap_or(ALL_TOOLS)` at lines 314-317 and 348-351 in `prompts.rs`.

The test helpers in `abstract_test_helpers.rs` explicitly set `mcp: Some(ALL_TOOLS.to_vec())` which is fine — tests should be explicit. The runtime fallback is the issue.