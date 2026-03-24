# Planning Report: Remove legacy compatibility, move label code to backend, remove Display

## Approach

Chose a simple colon-separated format (`"done"`, `"pending:main"`, `"running:main:bar"`) as the new API-level serialization, replacing both the label-prefix format (`"state:done, pipeline:main"`) and legacy milestone format (`"main_PENDING"`). This keeps the API clean — label representation stays in the GitHub backend only.

## Key design decisions

1. **`serialize_str()` instead of `Display`**: A named method makes the intent explicit and avoids implicit formatting. `{:?}` (Debug derive) is used for log output.
2. **Dependency order**: Rewrite serde first (step 1), then move constants (step 2), then fix callers (step 3) and tests (step 4). The compiler will catch all breakage from removed Display/PartialEq.
3. **No backward compatibility**: Per user request, both legacy milestone format and `"state:"` prefix format are removed from `From<&str>`. Old fs-backend YAML files with `"READY"` or `"main_PENDING"` will parse as `Unknown(...)`.

## Analog

The existing `Pipeline::MAIN`/`Pipeline::MERGE` + `as_str()` pattern in `task.rs` is the analog for the new `State::serialize_str()` method. The existing private prefix constants in `github.rs` (`STATE_PREFIX`, `PIPELINE_PREFIX`, etc.) are the analog for the relocated label constants.

## Files affected

- `zbobr-api/src/task.rs` — core State rewrite (Display, From, Serialize, PartialEq, constants, tests)
- `zbobr-task-backend-github/src/github.rs` — receives label constants from API
- `zbobr-task-backend-fs/src/fs.rs` — 3 changes (doc, serialize_str, is_done)
- `zbobr-api/src/backend.rs` — 2 doc comment updates
- `zbobr-dispatcher/src/cleanup.rs` — 1 change (is_done)
- `zbobr-dispatcher/src/prompts.rs` — 1 change (State::Ready)
- `zbobr-dispatcher/src/cli.rs` — 2 changes (debug format, serialize_str)
- `zbobr-dispatcher/src/workflow.rs` — 1 change (debug format)
- `zbobr/src/commands.rs` — 1 change (default_value)
- `zbobr-dispatcher/tests/mcp_integration/abstract_test_helpers.rs` — ~21 assertion updates
- `zbobr-dispatcher/tests/mcp_integration/test_helpers.rs` — ~4 assertion updates
- `zbobr-dispatcher/tests/mcp_integration/env.rs` — 2 assertion updates
