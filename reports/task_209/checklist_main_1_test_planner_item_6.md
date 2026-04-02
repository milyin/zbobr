# Test: build_executor_unknown_executor_error

**File:** `zbobr-dispatcher/src/lib.rs` (in existing `#[cfg(test)] mod tests`)

**Why:** The review fix (ae85fd88) changed `build_executor()` from `-> Box<dyn ToolExecutor>` to `-> anyhow::Result<Box<dyn ToolExecutor>>` and replaced the catch-all `_ => ClaudeExecutor` fallback with an explicit `"claude"` arm + `other => anyhow::bail!(...)`. No test was added for this error path. While `validate()` should normally catch bad executors first, `build_executor` is a public method and defense-in-depth warrants its own test.

**Test case:**
- Construct a `ResolvedProvider` with `executor = "nonexistent"`
- Use the existing `make_dispatcher` helper to build a minimal dispatcher
- Call `dispatcher.build_executor(&provider, None)`
- Assert result is `Err` and message contains "Unknown executor"

The test infrastructure (mock backends, `make_dispatcher` helper) already exists in the module.