## Scope / diff inspected
- Compared `origin/main...HEAD`.
- Changes are confined to **one file**: `zbobr-task-backend-github/src/github.rs`.
- Net: **69 insertions, 121 deletions**.

## Task requirement coverage
**Task:** move `flag:confirm` and `flag:pause` from labels to parameters; no backward-compat required.

✅ Implemented:
- Reading flags: `issue_to_task` now derives `pause`/`confirm` from the parsed PARAMETERS map (`FLAG_PAUSE`, `FLAG_CONFIRM`) rather than GitHub labels.
- Writing flags: `task_to_string_params` now emits `pause: true` / `confirm: true` (via `FLAG_VALUE_TRUE`) when the booleans are set.
- Removed label-based infra: `apply_flag_change`, `flag_to_label`, `label_to_flag`, and flag-label setup logic were removed.
- Legacy cleanup: `apply_state_change` now removes labels with `flag:` prefix, and `modify_task_internal` calls `apply_state_change` on **every save**, ensuring cleanup happens even when state doesn’t change.

## Analog / pattern consistency
The chosen analog (pipeline/stage already stored in PARAMETERS rather than labels) is appropriate. The new flag handling follows the same pattern: parse promoted fields from `parse_description_full`’s params map and serialize them back via `task_to_string_params`.

## Code quality & correctness notes
### ✅ Strengths
- Avoids repeated string literal for boolean by introducing `const FLAG_VALUE_TRUE: &str = "true";` and using it consistently.
- Eliminates dead code and reduces GitHub label surface area.
- Tests were updated to validate the new parameter-based behavior (reads pause/confirm from params, writes params when flags set).

### ⚠️ Non-blocking suggestions
1) **Parameter key naming vs `flag:` prefix**
   - Current param keys are `pause` / `confirm` (no `flag:` prefix). Given `separator::parse_parameters` splits at the *first* `:`, parameter keys cannot safely contain `:` without changing the parser. So this choice is defensible, but it may be worth documenting/renaming constants to make intent clearer (e.g., `FLAG_PARAM_PAUSE`, `FLAG_PARAM_CONFIRM`) to avoid confusion with label names.

2) **Boolean parsing strictness**
   - Parsing currently treats only exact `"true"` as true. If humans may edit PARAMETERS, consider accepting common variants (`True/TRUE/1/yes`) or documenting that only `true` is accepted.

3) **Legacy cleanup scope**
   - Cleanup removes **any** `flag:*` label on save. If the repo might have other user-defined `flag:*` labels, this is broad. If you want to be conservative, restrict removal to the known legacy labels (pause/confirm) rather than the entire prefix.

4) **Repo-level label cleanup**
   - `setup()` no longer deletes or manages `flag:*` labels, so legacy `flag:pause` / `flag:confirm` labels may remain defined in the repo. This matches the plan item “remove flag label management”, but if the goal is to fully remove them, you could explicitly delete those exact labels during setup.

## Extraneous changes
- None detected; all edits are directly tied to the migration and cleanup.

## Checklist assessment
All checklist items described in the task context appear completed, including the follow-up to ensure legacy flag label cleanup runs on every save.
