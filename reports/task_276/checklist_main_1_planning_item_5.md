## What

Fix the 2 integration test functions in `zbobr-dispatcher/tests/mcp_integration/env.rs` that construct `ZbobrDispatcherConfig` with a `tool` field:

1. **`init_fs_fs`** (around line 89) — Remove `tool: "mcp-tester".to_string()` from the struct literal. Ensure the tool is instead set on the role definitions used in the test's workflow config (if not already).

2. **`init_github_github`** (around line 227) — Same fix as above.

## Why

The `tool` field was removed from `ZbobrDispatcherConfig`. The tool should be specified at the role level in the workflow configuration. Check the test's workflow setup to ensure each role has a `tool` defined (which will now be required by validation).