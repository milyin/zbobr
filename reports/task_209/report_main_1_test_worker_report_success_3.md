## Tests Implemented

### Commit: 52631f6b

Added 2 tests to `zbobr-dispatcher/src/lib.rs` in the `mod tests` block:

1. **`validated_catches_circular_providers`** — Creates two providers with circular parent references (`a→b`, `b→a`), builds a dispatcher, calls `validated()`, and asserts it returns an error containing "circular".

2. **`validated_catches_invalid_workflow_refs`** — Creates a valid provider/tools config but a workflow with a role referencing tool "nonexistent". Uses `Workflow::from_config()` to bypass workflow validation and `make_dispatcher_with_workflow()` helper. Calls `validated()` and asserts the error mentions "nonexistent".

### Supporting changes
- Added `make_dispatcher_with_workflow()` helper that accepts a custom `Workflow` (the existing `make_dispatcher` delegates to it with `Workflow::default()`).
- Added imports for `RoleDefinition` and `WorkflowConfig` in the test module.

### Test results
All 67 unit tests in zbobr-dispatcher pass, including the 2 new ones.