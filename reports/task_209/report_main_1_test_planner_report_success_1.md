# Test Plan: Coverage for Review Fix (ae85fd88)

## Summary

The review fix commit addressed two correctness issues (priority inheritance and executor validation). The fix updated existing tests to match the new `Option<i32>` type but did **not** add tests for the three new code paths introduced. 3 additional tests are needed.

## Existing Coverage (20 tests, all passing)

- `resolve_providers`: 5 tests — basic, single inheritance, multi-level chain, circular ref, child overrides parent
- `validate`: 4 tests — valid config, unknown parent, no executor/no parent, unknown provider ref
- `resolve_tool_name`: 4 tests — stage precedence, role fallback, global fallback, no role fallback
- `select_provider`: 7 tests — basic, priority, round-robin, excluded skip, fallback to lower priority, all excluded error, unknown tool error

## Gaps Found (3 tests)

### 1. `resolve_providers_inherits_priority_from_parent` (config.rs)
Parent has `priority: Some(3)`, child has `priority: None` → child should resolve to 3, not default 10. This directly validates the `Option<i32>` fix. Current tests only cover same-default and explicit-override cases.

### 2. `validate_unknown_executor` (config.rs)
Provider with `executor = "invalid_executor"` → `validate()` should return error containing "unknown executor". Tests the new validation block at lines 634-643.

### 3. `build_executor_unknown_executor_error` (lib.rs)
`build_executor()` called with `ResolvedProvider { executor: "nonexistent", .. }` → should return `Err`. Tests the new explicit error path that replaced the `_ => ClaudeExecutor` fallback.

## Not Needed

- **Tool/Model newtype tests**: Existing tests in `task.rs` and `stage_title.rs` already cover serde and display.
- **Exclusion expiry test**: Would require time mocking (`Instant`-based), complexity not justified for a simple `retain` call.
- **Integration-level cli.rs tests**: The stage runner wiring is integration-level; the unit-testable logic (`select_provider`, `build_executor`, `resolve_tool_name`) is already covered.
- **init.rs template**: Template correctness is a documentation concern, not a logic test.