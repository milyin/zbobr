## Scope / diff inspected
- Compared branch to `origin/main` (`git log origin/main..HEAD`, `git diff origin/main...HEAD`).
- Changed files:
  - `zbobr-api/src/task.rs`, `zbobr-api/src/lib.rs`
  - `zbobr-task-backend-github/src/github.rs`
  - `zbobr-task-backend-fs/src/fs.rs`
  - `zbobr-dispatcher/src/prompts.rs`

## Task requirements coverage
### Move `flag:pause` / `flag:confirm` from labels → PARAMETERS (no backward-compat reads)
- ✅ `issue_to_task` now derives `task.pause` and `task.confirm` from `params_map` via `PARAM_FLAG_PAUSE` / `PARAM_FLAG_CONFIRM` and compares to canonical `PARAM_FLAG_VALUE_TRUE`.
- ✅ No remaining reads of `flag:pause` / `flag:confirm` labels; search shows no matches.

### Remove flag label infra / cleanup
- ✅ GitHub backend no longer creates/manages flag labels.
- ✅ Legacy flag labels are removed during `apply_state_change` by stripping any label with `FLAG_LABEL_PREFIX` (`"flag:"`), and `modify_task_internal` always calls `apply_state_change(...)` so cleanup happens on every save.
  - Note: Keeping `FLAG_LABEL_PREFIX` is justified purely for deletion of legacy data (not compatibility parsing).

### “Avoid repeated literals” for parameter names
- ✅ Parameter-key string literals have been centralized as `pub const PARAM_*` in `zbobr-api/src/task.rs` and re-exported from `zbobr-api/src/lib.rs`.
- ✅ Call sites (GitHub backend + dispatcher prompts + FS backend where applicable) use the constants.
- ✅ Repository-wide search indicates the parameter-key literals (e.g. `"destination_repository"`, `"pipeline_run_id"`, etc.) now appear only in the canonical constant definitions (plus an unrelated serde alias for `stage`).

## Analog choice & consistency
- Analog chosen (existing PARAM-based fields like `pr_url`, pipeline/stage/signal/stack) is appropriate.
- New flag params follow the same serialization/parsing pattern as other params, using a shared constant for the canonical `true` value.

## Code quality / robustness notes
- ✅ Good: central constants reduce drift risk, and `apply_state_change` cleanup being unconditional prevents legacy label persistence.
- Minor style nit (non-blocking): a couple of long lines in `issue_to_task` (`pause` / `confirm` assignments) look un-rustfmt’d; consider running `cargo fmt` if formatting is enforced in CI.
- Possible enhancement (non-blocking): if future boolean-like params are added, consider a tiny helper like `fn param_is_true(map, key) -> bool` to keep parsing consistent.

## Extraneous changes
- None found; changes are directly related to migrating flags to params and de-literalizing parameter keys.

## Overall
Implementation matches the plan/checklist, removes label-based flags in favor of PARAMETERS, and systematically replaces parameter-name literals with canonical `PARAM_*` constants.