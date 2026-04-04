# Tests: linting and linter_worker stage transition routing

## Location
`zbobr/src/init.rs` — `#[cfg(test)] mod tests`

## What to test
Read the `linting` and `linter_worker` stage definitions from `default_workflow()` and assert that their `on_success` / `on_failure` fields target the correct stages.

### Test 1: linting_on_success_routes_to_testing
```rust
#[test]
fn linting_on_success_routes_to_testing() {
    let wf = default_workflow();
    let main = wf.pipelines.get(&Pipeline::Main).unwrap();
    let linting = main.stages.get(&Stage::from("linting")).unwrap();
    let target = linting.on_success().and_then(|t| t.next.as_deref());
    assert_eq!(target, Some("testing"));
}
```

### Test 2: linting_on_failure_routes_to_linter_worker
```rust
#[test]
fn linting_on_failure_routes_to_linter_worker() {
    let wf = default_workflow();
    let main = wf.pipelines.get(&Pipeline::Main).unwrap();
    let linting = main.stages.get(&Stage::from("linting")).unwrap();
    let target = linting.on_failure().and_then(|t| t.next.as_deref());
    assert_eq!(target, Some("linter_worker"));
}
```

### Test 3: linter_worker_on_success_routes_to_linting
```rust
#[test]
fn linter_worker_on_success_routes_to_linting() {
    let wf = default_workflow();
    let main = wf.pipelines.get(&Pipeline::Main).unwrap();
    let lw = main.stages.get(&Stage::from("linter_worker")).unwrap();
    let target = lw.on_success().and_then(|t| t.next.as_deref());
    assert_eq!(target, Some("linting"));
}
```

### Test 4: linter_worker_on_failure_routes_to_working
```rust
#[test]
fn linter_worker_on_failure_routes_to_working() {
    let wf = default_workflow();
    let main = wf.pipelines.get(&Pipeline::Main).unwrap();
    let lw = main.stages.get(&Stage::from("linter_worker")).unwrap();
    let target = lw.on_failure().and_then(|t| t.next.as_deref());
    assert_eq!(target, Some("working"));
}
```

## Rationale
These tests directly encode the routing contract introduced by this feature. They would have caught the lint-loop regression (missing `linting.on_success = testing`) as a compile-time/test failure rather than a runtime loop.
