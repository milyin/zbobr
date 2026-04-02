# Tests for `select_provider()` and `exclude_provider()` — zbobr-dispatcher/src/lib.rs

These tests validate the provider selection logic: priority-based filtering, round-robin within a tier, and temporary exclusion handling.

## Approach

Add `#[cfg(test)] mod tests` inside `zbobr-dispatcher/src/lib.rs`. Since the module has private access, we can construct `ZbobrDispatcher` directly (bypassing the builder) with minimal mock backends.

Create minimal mock structs `MockTaskBackend` (impl `TaskBackend`) and `MockRepoBackend` (impl `WorktreeBackend`) that panic on all methods (they won't be called). These are only needed to satisfy the struct fields.

## Test cases

### 1. `select_provider_basic`
Config with tool "smart" having one entry `{ provider: "claude", model: "opus" }` and provider "claude" with executor "claude". Assert `select_provider("smart")` returns the resolved provider with executor "claude" and model "opus".

### 2. `select_provider_prefers_higher_priority`
Two providers: "claude" priority 10, "fallback" priority 0. Tool "smart" has entries for both. Assert that `select_provider` picks "claude" (priority 10).

### 3. `select_provider_round_robin_same_priority`
Two providers both at priority 10. Tool entries reference both. Call `select_provider` twice — assert different providers returned (round-robin).

### 4. `select_provider_skips_excluded`
Two providers at same priority. Exclude one via `exclude_provider()`. Assert `select_provider` returns the non-excluded one.

### 5. `select_provider_falls_back_to_lower_priority_when_higher_excluded`
Provider A at priority 10, provider B at priority 0. Exclude A. Assert `select_provider` returns B.

### 6. `select_provider_all_excluded_error`
Single provider. Exclude it. Assert `select_provider` returns an error mentioning "excluded".

### 7. `select_provider_unknown_tool_error`
Call `select_provider("nonexistent")`. Assert error mentioning "not found".

## Location

`zbobr-dispatcher/src/lib.rs` — new `#[cfg(test)] mod tests { ... }` at the end of file.

## Dependencies

Requires mock backend implementations (minimal, panic-on-call stubs). Also needs `ZbobrDispatcherConfig` with providers/tools populated, `Workflow::default()`, and default executor configs.
