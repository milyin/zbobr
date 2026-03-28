# Plan: Move flag:confirm and flag:pause from Labels to Parameters

## Approach
All changes are isolated to `zbobr-task-backend-github/src/github.rs`. The analog for this migration is how `pipeline`, `stage`, `signal`, and other fields were already moved to params — read from `params_map` in `issue_to_task`, written in `task_to_string_params`.

## Key Design Decisions
- **No backward compatibility**: old `flag:*` labels on existing issues will simply be ignored after the change.
- **Param key constants**: `PARAM_PAUSE` and `PARAM_CONFIRM` constants replace the old `FLAG_PAUSE`/`FLAG_CONFIRM` constants, satisfying the user's requirement to avoid string literals.
- **Omit-when-false serialization**: consistent with other optional params (e.g., `pr_url`), the flags are only written to params when `true`.
- **Full cleanup**: all flag-label infrastructure (creation in `configure_repo`, `apply_flag_change`, helper methods, obsolete constants) is removed.

## Checklist Summary
1. Read pause/confirm from params_map in issue_to_task (add PARAM_PAUSE/PARAM_CONFIRM constants)
2. Write pause/confirm to params in task_to_string_params
3. Remove apply_flag_change method and its save_task call
4. Remove flag label management from configure_repo
5. Remove unused constants and helpers (FLAG_PREFIX, FLAG_PAUSE, FLAG_CONFIRM, ALL_FLAG_NAMES, flag_to_label, label_to_flag)
6. Remove/update the apply_flag_change test