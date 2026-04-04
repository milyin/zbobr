Implementation is **not ready to accept**. The prompt cleanup looks good now, and the scope is tight, but the workflow wiring introduced a blocking regression.

## Overall assessment

The structural direction is correct:
- `linting.on_failure` now routes to `linter_worker`
- `linter_worker.on_success` routes back to `linting`
- `linter_worker.on_failure` escalates to `working`
- the new role registration and prompt-file registration are consistent with the intended dedicated fixer-stage design
- the earlier prompt-policy issues appear resolved in the current branch state

However, the implementation missed an important detail about how stage progression works in this codebase.

## Blocking finding

### 1. Successful `linting` now falls through to `linter_worker`
**Files:**
- `zbobr/src/init.rs:332-347`
- `zbobr-dispatcher/src/workflow.rs:173-199`
- `zbobr-api/src/config.rs:222-227`

`PipelineConfig::next_stage()` uses the **ordered next stage** as the default success transition when `on_success` is not set. `sequential_signal()` in the dispatcher applies that fallback for `ReportSuccess`.

Because this change inserts `linter_worker` immediately after `linting` but does **not** set `linting.on_success`, a successful lint run now advances to `linter_worker` instead of `testing`.

That creates the wrong behavior:
- `linting` passes
- pipeline advances to `linter_worker`
- `linter_worker.on_success` sends the task back to `linting`
- the task loops between `linting` and `linter_worker`
- `testing` is no longer reached on the normal success path

This directly contradicts the task requirement. The new stage should only handle lint failures, not successful lint runs.

**Required fix:** explicitly set `linting.on_success` to `testing` (or otherwise preserve the original success path while keeping failure -> `linter_worker`).

## Analog consistency

The chosen analog (`test_worker`) was only partially applied correctly:
- **Good:** a dedicated fixer stage/role is the right pattern.
- **Problem:** unlike `test_planner -> test_worker`, the `linting` stage does not want its normal success path to flow into the new helper stage. Because this workflow engine uses stage order as the default success transition, inserting the new stage before `testing` requires an explicit `on_success` override.

So the analog was reasonable for the dedicated worker stage, but the implementation did not account for the workflow engine behavior that makes order semantically significant.

## Scope / necessity of changes

The branch diff is narrowly scoped to `zbobr/src/init.rs`, and I did not find unrelated changes in the task diff.

## Checklist status

There were no unchecked checklist items left in context to evaluate. The listed items are structurally present, but the implementation still must be rejected until the success-path regression above is fixed.