# Test Plan: linter_worker Stage

## Summary
The implementation adds a `linter_worker` stage as a dedicated fixing stage between `linting` and `testing`. The critical routing change is:
- `linting` success → `testing` (explicit, was previously implicit/missing)
- `linting` failure → `linter_worker` (new)
- `linter_worker` success → `linting` (loop-back to verify)
- `linter_worker` failure → `working` (escalation)

No existing tests cover `default_workflow()` routing or `PROMPT_FILES` completeness, leaving these behavioral contracts unverified.

## Tests Required

### 1. default_workflow() passes validate() [ctx_rec_17]
A single call to `default_workflow().validate()` in `zbobr/src/init.rs` tests. This acts as a structural integrity check that catches unknown stage references in transitions.

### 2. linting and linter_worker transition routing [ctx_rec_18]
Four focused unit tests in `zbobr/src/init.rs` tests asserting the exact `on_success`/`on_failure` targets for both `linting` and `linter_worker` stages. These encode the routing contract and would have caught the lint-loop regression.

### 3. PROMPT_FILES completeness [ctx_rec_19]
One test verifying that every role in `default_workflow()` that specifies a `prompt` path has a matching entry in `PROMPT_FILES`. Prevents silent missing-prompt bugs when new roles are added.

## Tests NOT Required
- Snapshot/content assertions on `LINTER_PROMPT` or `LINTER_WORKER_PROMPT` text — these are authoring artifacts, not API contracts.
- Integration-level tests for the new stage — the existing abstract pipeline tests already cover failure-routing and loop-back patterns generically; the unit tests above are sufficient to verify the specific wiring.
