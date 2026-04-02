# Test Plan: Separate Executor Settings with Fallbacks

## Analysis

The implementation introduces significant new logic with **zero existing unit tests** in the two most-changed files:

- **`zbobr-api/src/config.rs`** (0 tests) — new `resolve_providers()`, `resolve_tool_name()`, and updated `validate()` methods
- **`zbobr-dispatcher/src/lib.rs`** (0 tests) — new `select_provider()` and `exclude_provider()` methods

The existing tests in `task.rs` and `stage_title.rs` were updated for the type changes (Tool/Model newtypes) and pass. No additional tests are needed there.

## Test Groups

### Group 1: `resolve_providers()` — 5 tests (zbobr-api/src/config.rs)
Pure function tests for provider inheritance resolution:
- Basic provider (no parent)
- Single-level inheritance
- Multi-level inheritance chain
- Circular reference detection (error)
- Child field override precedence

### Group 2: `resolve_tool_name()` — 4 tests (zbobr-api/src/config.rs)
Tool name resolution precedence chain:
- Stage tool takes precedence
- Falls back to role tool
- Falls back to global dispatcher tool
- No role defined → global fallback

### Group 3: `validate()` — 4 tests (zbobr-api/src/config.rs)
Configuration consistency checks:
- Valid config passes
- Unknown parent reference → error
- Provider with neither executor nor parent → error
- Tool referencing unknown provider → error

### Group 4: `select_provider()` — 7 tests (zbobr-dispatcher/src/lib.rs)
Provider selection logic with mock backends:
- Basic selection
- Higher-priority provider preferred
- Round-robin within same priority tier
- Excluded providers skipped
- Falls back to lower priority when higher excluded
- All excluded → error
- Unknown tool → error

## Implementation Notes

- Groups 1–3 are straightforward unit tests using plain struct construction — no mocking needed.
- Group 4 requires minimal mock `TaskBackend`/`WorktreeBackend` stubs (panic-on-call) to construct `ZbobrDispatcher`. Tests go in the same module for private field access.
