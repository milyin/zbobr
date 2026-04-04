## Fix applied

The blocking regression identified in review has been fixed in commit `f9797396`.

### Problem
In `zbobr-dispatcher/src/prompts.rs`, both `build_full_prompt()` (line 317) and `build_prompt_with_task()` (line 351) used `.unwrap_or(&[])` when a role's `mcp` field was `None`. This meant roles without an explicit `mcp` list would get **no** MCP tool variables injected into their prompts, breaking existing configs that relied on the implicit "all tools" default.

### Fix
- Changed `.unwrap_or(&[])` to `.unwrap_or(ALL_TOOLS)` in both functions
- Added `ALL_TOOLS` to the import from `zbobr_api::config_tools`
- This restores the previous behavior: `None` means "all tools", `Some(vec![])` means "no tools"

### Verification
- `cargo check` passes cleanly
- All 94 `zbobr-dispatcher` lib tests pass
- All 126 `zbobr-api` lib tests pass
- 3 pre-existing failures in `zbobr-task-backend-github` are unrelated (CryptoProvider configuration issue in test environment)