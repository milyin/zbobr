## Summary

Fixed the two correctness issues found in the review (ctx_rec_36).

### Fix 1: Provider cycles caught at startup (ctx_rec_37)

Added `self.config.resolve_providers()?` call in `ZbobrDispatcher::validated()` (zbobr-dispatcher/src/lib.rs). This forces the full provider inheritance graph to be resolved at startup, so circular chains like `a -> b -> a` are rejected immediately with a clear error message instead of failing mid-execution when a stage first uses one of those providers.

### Fix 2: Provider exclusion only on connectivity failures (ctx_rec_38)

Added `connectivity_failure: bool` field to `SessionOutcome` in zbobr-dispatcher/src/cli.rs. The field is set `true` only in the `Err(e)` arm of `execute_tool()` — spawn/IO failures where no output is produced, indicating the provider itself was unreachable. It stays `false` for `exit_ok: false` (non-zero exit), which represents a task-level failure unrelated to provider availability. The exclusion at line 575 was updated to check `outcome.connectivity_failure` instead of `outcome.execution_error.is_some()`.

### Tests

All 241 tests continue to pass (1 commit: 5155f47f).