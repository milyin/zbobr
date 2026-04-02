## Test Plan for Review Fix (commit 5155f47f)

### Changes analyzed

Two fixes from the latest review:
1. **Provider cycles caught at startup** — `resolve_providers()` and `validate_workflow_refs()` added to `validated()` in zbobr-dispatcher/src/lib.rs
2. **Provider exclusion only on connectivity failures** — New `connectivity_failure: bool` field on private `SessionOutcome` in zbobr-dispatcher/src/cli.rs

### Existing coverage

- `resolve_providers_circular_reference` in zbobr-api/src/config.rs — tests cycle detection in the function itself
- `validate_workflow_refs_*` (4 tests) in zbobr-api/src/config.rs — tests the function itself
- `select_provider_*` (7 tests) in zbobr-dispatcher/src/lib.rs — tests provider selection and exclusion
- Integration tests in zbobr-dispatcher/tests/ — exercise full pipeline

### Gap: `validated()` wiring not tested

The underlying functions are tested in isolation, but no test verifies that `validated()` actually calls them. A circular provider config or invalid workflow ref would silently pass `make_dispatcher` in tests.

### Tests to add (1 checklist item, 2 tests)

| # | Test name | File | What it covers |
|---|-----------|------|----------------|
| 1 | `validated_catches_circular_providers` | zbobr-dispatcher/src/lib.rs | `validated()` eagerly calls `resolve_providers()` → catches `a -> b -> a` cycle |
| 2 | `validated_catches_invalid_workflow_refs` | zbobr-dispatcher/src/lib.rs | `validated()` calls `validate_workflow_refs()` → catches role referencing unknown tool |

### Not requiring tests: `connectivity_failure`

The `connectivity_failure` field is on the private `SessionOutcome` struct. Testing the four arms of `execute_tool()` would require:
- A mock `ToolExecutor` that doesn't exist
- Handling `tokio::select!` with `ctrl_c()` in tests

The field assignments are trivially correct by inspection (each match arm sets a boolean literal). Creating mock infrastructure for this is disproportionate to the regression risk. The behavioral change is a single-line condition (`outcome.connectivity_failure` vs `outcome.execution_error.is_some()`) covered by integration tests.