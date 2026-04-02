## Tests for `validated()` integration in zbobr-dispatcher/src/lib.rs

These tests verify the two new checks added to `ZbobrDispatcher::validated()` in commit 5155f47f. The underlying functions (`resolve_providers`, `validate_workflow_refs`) are already unit-tested in zbobr-api/src/config.rs, but the wiring through `validated()` is not tested.

### Test 1: `validated_catches_circular_providers`

**Location**: `zbobr-dispatcher/src/lib.rs`, `mod tests` block  
**What it tests**: The `validated()` method eagerly calls `self.config.resolve_providers()?`, so a dispatcher with circular provider inheritance (`a -> b`, `b -> a`) should fail during `validated()` with an error containing "circular".

**Setup**:
- Create two `ProviderDefinition`s: `a` with `parent = Some("b")` and `b` with `parent = Some("a")` — neither has an executor.
- Create a tools map with one tool referencing provider `a`.
- Build the dispatcher with `make_dispatcher()`.
- Call `.validated()` and assert it returns `Err` with "circular" in the message.

### Test 2: `validated_catches_invalid_workflow_refs`

**Location**: `zbobr-dispatcher/src/lib.rs`, `mod tests` block  
**What it tests**: The `validated()` method calls `self.config.validate_workflow_refs(self.workflow.config())?`, so a dispatcher whose workflow references a tool not in the config's `[tools]` map should fail at `validated()`.

**Setup**:
- Create a valid provider and tools map (e.g., tool "smart" with one entry).
- Create a `WorkflowConfig` with a role that has `tool = Some("nonexistent")`.
- Build the dispatcher using `Workflow::from_config(workflow_config)` to bypass workflow validation.
- Call `.validated()` and assert it returns `Err` mentioning "unknown tool" or the nonexistent tool name.

### Why no tests for `connectivity_failure`

The `connectivity_failure` field lives on the private `SessionOutcome` struct in `cli.rs`. Testing the four arms of `execute_tool()` would require a mock `ToolExecutor` implementation and handling `tokio::select!` with `ctrl_c()`. The field assignments are simple booleans, easily verified by inspection. Creating mock infrastructure for this is disproportionate to the regression risk.