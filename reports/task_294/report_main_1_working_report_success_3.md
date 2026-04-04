## Fix: Linting stage success path regression

### Problem
The `linting` stage was missing an explicit `on_success` transition. Since the workflow engine uses the next stage in order as the default success transition, a successful lint run would advance to `linter_worker` (inserted immediately after `linting`) instead of `testing`. This created an infinite loop: `linting` → `linter_worker` → `linting` → ...

### Fix
Added `on_success: Some(StageTransition::stage("testing"))` to the `linting` stage definition in `zbobr/src/init.rs`.

### Final stage wiring
- `linting` success → `testing` (explicit)
- `linting` failure → `linter_worker` (explicit)
- `linter_worker` success → `linting` (loop back to verify)
- `linter_worker` failure → `working` (escalate)

### Verification
`cargo build` passes with no errors.