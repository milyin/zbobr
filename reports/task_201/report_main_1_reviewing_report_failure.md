## Scope / diff reviewed
Compared `origin/main...HEAD`.

Changed files:
- `zbobr-task-backend-github/src/github.rs`
- `zbobr-api/src/backend.rs`
- `zbobr-dispatcher/src/lib.rs`, `zbobr-dispatcher/src/backend.rs`, `zbobr-dispatcher/src/task.rs`
- `zbobr-task-backend-fs/src/fs.rs`

## Task requirements check
✅ Core requirement is implemented:
- `pipeline`, `stage`, `signal` are now stored in issue params (description body) instead of labels.
- Only `state:*` and `flag:*` remain label-driven.
- Signal label management and `apply_signal_change()` are removed.
- No backward-compat was attempted (consistent with request).

## Analog / pattern consistency
Good analog choice: the existing param mechanism used for `pipeline_run_id` via `task_to_string_params()` + `parse_description_full()/serialize_description_full()` is reused for `pipeline`/`stage`/`signal`. The GitHub backend changes follow the same structure as the pre-existing param plumbing.

## Issues to fix (blocking)
### 1) Outdated / misleading documentation
- `zbobr-api/src/backend.rs` trait docs still say GitHub uses “signals/tools/models as Labels”. Signal is no longer a label in this branch.
  - This is now actively misleading and should be updated to match reality (e.g. signal/pipeline/stage in params; state/flags in labels).

### 2) Stale comment in `apply_state_change`
- `zbobr-task-backend-github/src/github.rs` line ~353:
  - Comment says it removes `state:/pipeline:/stage:` labels, but the code now removes only `state:`.
  - Update the comment to avoid future regressions/confusion.

### 3) Robustness: empty/invalid params can produce malformed State
New `labels_to_state(labels, pipeline_param, stage_param)` currently accepts empty strings:
- `pipeline_param = params_map.get("pipeline").map(|s| s.as_str())`
- `stage_param = params_map.get("stage").map(|s| s.as_str())`

If a param exists but is empty/whitespace (manual edit, partial update, etc.), code will construct:
- `Pipeline::Custom("")` and/or `Stage("")`, yielding `State::Pending(Pipeline::Custom(""))` / `State::Running(_, Stage(""))`.

This is less strict than the older label-based path and less strict than `State::from(&str)` which checks for non-empty parts.

**Suggested fix:** normalize/validate params before passing into `labels_to_state`, e.g.
- `let pipeline_param = params_map.get(KEY_PIPELINE).map(String::as_str).map(str::trim).filter(|s| !s.is_empty());`
- same for `stage_param`.
Optionally, do the validation inside `labels_to_state` for a single point of truth.

## Standards / maintainability findings (should fix)
### 4) Repeated string literals for new param keys
New keys `"pipeline"`, `"stage"`, `"signal"` are repeated in multiple places in `github.rs`.
Given the project rule “Avoid repeated string literals”, these should be centralized (e.g. `const PARAM_PIPELINE: &str = "pipeline";`, etc.).

This also helps prevent partial updates (changing a key in one place but not another).

## Non-blocking notes
- Existing repo signal/pipeline/stage labels are not cleaned up. That’s fine given “no backward compat” and the goal of reducing *future* label churn, but you may optionally remove obsolete labels in setup or document the migration.

## Overall assessment
Functionally the implementation matches the task goal (reduce label-change noise by moving pipeline/stage/signal to params). However, docs/comments are now inaccurate and `labels_to_state` should reject empty params to be resilient to partial/manual edits. Address the blocking items above before merging.