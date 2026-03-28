## Plan: Move flag:confirm and flag:pause from labels to parameters

### Context

The GitHub backend currently stores `pause` and `confirm` as GitHub issue labels (`flag:pause`, `flag:confirm`). The FS backend already stores them as first-class boolean fields. The task asks to align the GitHub backend with a param-based approach — the same pattern already used for `pipeline`, `stage`, and `signal`.

### Proposed Changes (GitHub backend only: `zbobr-task-backend-github/src/github.rs`)

1. **`issue_to_task`**: Read `pause` and `confirm` from `params_map` (e.g., `params_map.get("pause") == Some("true")`) instead of scanning labels.

2. **`task_to_string_params`**: Insert `pause` and `confirm` into the params map when `true` (following the existing convention of omitting falsy/default params).

3. **`apply_flag_change`**: Remove this method — flags are now part of the description body and don't need a separate label-sync step.

4. **`modify_task_internal`**: Remove the `apply_flag_change` call (line ~914). Flag changes are automatically handled by the description update already present in this method.

5. **`setup`**: Remove flag label creation/deletion:
   - Remove the loop that creates `flag:pause` / `flag:confirm` labels.
   - Remove `flag_labels` from `expected_labels` set used for obsolete label deletion.
   - Update the obsolete label cleanup to only target `state:*` labels (not `flag:*`).

6. **Constants/helpers cleanup**: Remove `FLAG_PREFIX`, `FLAG_PAUSE`, `FLAG_CONFIRM`, `ALL_FLAG_NAMES` constants; remove `flag_to_label` and `label_to_flag` helper methods.

7. **Tests**: Remove/update the commented-out tests (lines ~1432–1473) that reference the old label-based flag behavior.

### No changes needed

- FS backend already stores flags as fields.
- `Task` struct API (pause/confirm remain bool fields).
- All dispatcher, MCP, and CLI code — they work with the abstract `Task` struct, unaffected by storage format.
