## Problem

Integration test dispatcher configs in `zbobr-dispatcher/tests/mcp_integration/env.rs` and `abstract_test_helpers.rs` use `tool = "mcp-tester"` with the old semantic (executor name). The new production model requires:
1. `providers["mcp-tester"] = { executor = "mcp-tester" }` in the dispatcher config
2. `tools["mcp-tester"] = [{ provider = "mcp-tester", model = "test-model" }]` in the dispatcher config
3. Stage/dispatcher `tool` fields pointing to this tool name (unchanged)
4. Dispatchers built with `.validated()` to match production startup behavior

## Files to change
- `zbobr-dispatcher/tests/mcp_integration/env.rs`: add providers/tools to ZbobrDispatcherConfig, change `.build()` to `.build().validated().expect(...)`

## No changes needed
- `abstract_test_helpers.rs`: stage `tool: Some("mcp-tester".to_string())` already uses the correct tool name, and configs have providers/tools defined at env level