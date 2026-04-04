# Test: default_workflow() passes validate()

## Location
`zbobr/src/init.rs` — `#[cfg(test)] mod tests`

## What to test
Call `default_workflow()` and invoke `.validate()` on the resulting `WorkflowConfig`. Assert `Ok(())`.

This catches invalid stage-reference bugs in transitions (e.g., the regression where `linting.on_success` was missing, causing the workflow engine to fall back to advancing to `linter_worker` instead of `testing`).

## Example
```rust
#[test]
fn default_workflow_is_valid() {
    let workflow = default_workflow();
    assert!(workflow.validate().is_ok(), "default workflow must pass validation");
}
```

## Rationale
There is currently no test that validates the built-in default workflow config. The linting loop bug (linting → linter_worker → linting → …) was a silent routing error that no existing test would catch. Validating the default workflow ensures all stage-transition targets exist in the config.
