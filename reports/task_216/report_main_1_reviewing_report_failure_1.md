## Scope / diff checked
- Compared `origin/main...HEAD`.
- Changes are confined to `zbobr-task-backend-github/src/github.rs` (net -117/+67).

## Task requirements
> Move `flag:confirm` and `flag:pause` from GitHub labels to issue body PARAMETERS; no backward compatibility.

### What’s correct
- **Reading flags**: `issue_to_task()` now derives `pause`/`confirm` from `params_map.get(FLAG_{PAUSE,CONFIRM}) == FLAG_VALUE_TRUE` (no label parsing). ✅
- **Writing flags**: `task_to_string_params()` now inserts `pause: true` and `confirm: true` into PARAMETERS when the bools are set. ✅
- **Label infra removal**: `flag_to_label`, `label_to_flag`, `apply_flag_change`, flag-label creation in `setup()`, and flag-label inclusion in the “expected managed labels” set were removed. ✅
- **Avoid repeated literals**: introduced `FLAG_VALUE_TRUE` constant; flag names are consts. ✅
- **Tests updated**: label-based flag test removed; new tests cover param parsing + serialization. ✅

## Major issue (must fix)
### Legacy `flag:` labels are **not reliably cleaned up**
- The new “cleanup” was implemented by extending `apply_state_change()` to remove labels starting with `FLAG_LABEL_PREFIX`.
- However, `save_task()` calls `apply_state_change()` **only when `task.state != original_state`**.
- Result: if an issue still has legacy `flag:pause`/`flag:confirm` labels and you save a task where **only params change** (or you just want cleanup) while state remains unchanged, the legacy labels **will remain indefinitely**.

This contradicts the worker report claim “strip legacy flag: labels on save” and does not fully address the prior review finding.

#### Suggested fix
- Implement a dedicated helper, e.g. `async fn strip_legacy_flag_labels(&self, id: u64)` that:
  - fetches current labels
  - removes any label with `name.starts_with(FLAG_LABEL_PREFIX)` (ignore failures similarly to current removal)
- Call it from `save_task()` unconditionally (or at least when `task.pause/confirm` differ from the fetched/original task), independent of state changes.
- Keep `apply_state_change()` focused on state labels; if you keep legacy cleanup there too, still call the new helper from `save_task()` to ensure cleanup happens.

## Minor notes / polish
- `retry_github("remove state label", ...)` is now used for removing legacy flag labels too; consider renaming the op_name string to something neutral like `"remove issue label"` to avoid confusion (no functional impact).
- Naming clarity: `STATE_LABEL_PAUSE` and `FLAG_PAUSE` both equal `"pause"`. It’s correct, but consider renaming one side (e.g. `FLAG_PARAM_PAUSE`) to reduce cognitive collisions.

## Analog / consistency assessment
- The approach matches the existing pattern used for other promoted fields (pipeline/stage/signal stored in PARAMETERS). ✅
- The “no backward compatibility” requirement is respected (no label-based parsing). ✅

## Verdict
Failing due to incomplete legacy label cleanup: cleanup currently only occurs when state changes, not on save in general.