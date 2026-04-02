Both fixes are implemented and all tests pass:

1. **Cycle detection at startup**: `ZbobrDispatcher::validated()` now calls `resolve_providers()` eagerly, so a config like `a -> b -> a` is rejected at startup, not mid-execution.

2. **Targeted provider exclusion**: Added `connectivity_failure: bool` to `SessionOutcome`. Only spawn/IO errors (`Err(e)` arm) set it true. Non-zero exit codes are task-level failures and no longer poison the provider. The exclusion check now uses `outcome.connectivity_failure`.