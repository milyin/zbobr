## Summary

Fixed the last review finding: integration test dispatcher configs were not aligned with the production provider/tool model.

## Changes

### `zbobr-dispatcher/tests/mcp_integration/env.rs`
- Added `indexmap::IndexMap`, `zbobr_api::{Model, config::{ProviderDefinition, ToolEntry}}` imports
- Added `test_providers_and_tools()` helper that creates a `"mcp-tester"` provider (executor = "mcp-tester") and a `"mcp-tester"` tool entry (provider = "mcp-tester", model = "test-model")
- Both `init_fs_fs` and `init_github_github` now populate `providers` and `tools` in `ZbobrDispatcherConfig`
- All four dispatcher builder chains (2 default + 2 factory closures) now call `.build().validated().expect("test dispatcher config must be valid")`

### `zbobr-dispatcher/tests/mcp_integration/abstract_test_helpers.rs`
- Removed unused `task::Tool` import (was previously used but is no longer needed after the provider/tool refactor)

## Result

All 81 tests pass (67 unit + 14 integration). No new warnings introduced.