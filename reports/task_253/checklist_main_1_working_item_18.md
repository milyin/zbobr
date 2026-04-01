Remove all remaining preparator references from public interfaces:

1. `zbobr-executor-mcp-tester/src/config.rs`: remove `pub preparation: Option<PathBuf>` field and "preparation" | "preparator" match arm in `scenario_for_stage()`
2. `zbobr-dispatcher/tests/mcp_integration/test_helpers.rs`: remove `run_preparation()` function (uses "preparator" stage)
3. `zbobr-dispatcher/src/config.rs`: delete the entire `/* ... */` commented-out block containing old preparator tests
4. `zbobr-dispatcher/src/cli.rs:1687`: remove stale comment about "preparator stage"
5. `zbobr-api/src/tool_executor.rs:51`: update doc comment that mentions "non-Preparator roles"