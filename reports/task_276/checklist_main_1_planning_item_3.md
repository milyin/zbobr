## What

Fix the 3 test functions in `zbobr-api/src/config.rs` that set or read `config.tool` on `ZbobrDispatcherConfig`, which no longer exists:

1. **`validate_rejects_unknown_global_tool`** — Remove this test entirely. The global tool concept no longer exists.

2. **`validate_rejects_when_tools_empty`** — Remove this test entirely. It tests validation of a global tool against an empty tools map, which is no longer relevant.

3. **`validate_passes_when_global_tool_exists`** — Remove this test entirely. The global tool concept no longer exists.

## Why

These tests validated behavior of a global `tool` field that has been removed from `ZbobrDispatcherConfig`. They serve no purpose and cause compilation errors.