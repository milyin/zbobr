## Scope / diff
Only `zbobr-task-backend-github/src/github.rs` changed.

## What looks good
- Flag parsing moved from GitHub labels to PARAMETERS: `issue_to_task` now reads `pause`/`confirm` from `params_map` and `task_to_string_params` writes them when set.
- All explicit flag-label infrastructure is removed (prefix, conversion helpers, label creation/update), consistent with the plan to stop using labels.
- No remaining `flag:` references elsewhere in the repo (grep clean).

## Blocking issues (should fix)
### 1) “Remove flag labels” is incomplete: no cleanup of existing `flag:*` labels
The implementation stops *reading/writing* flag labels, but it also stops *removing* them. As a result:
- Existing repos will keep the `flag:pause` / `flag:confirm` labels forever (setup no longer deletes `flag:*`).
- Existing issues that already have `flag:*` labels will keep them even after the task is saved/updated, because `modify_task_internal` only calls `apply_state_change` (state labels) and never removes flag labels anymore.

Given the task is explicitly “remove flag labels” and we’re allowed to break backward compatibility, the backend should proactively remove these labels (repo-level + issue-level), e.g.:
- In `setup(force)` (or `configure_repo` equivalent), delete the known `flag:pause` and `flag:confirm` labels (and possibly any `flag:*` labels) when present.
- In the save path (`modify_task_internal`), remove any `flag:*` labels from the issue to avoid persistent stale UI state.

Relevant areas:
- Setup label cleanup currently only considers `state:` labels.
- Save path: `modify_task_internal` applies state label changes only.

### 2) Repeated string literal "true" (project rule: avoid repeated literals)
New flag-param parsing/serialization uses the literal `"true"` in multiple places:
- parsing: `params_map.get(FLAG_PAUSE).map(|s| s == "true")...`
- serialization: `params.insert(..., "true".to_string())`

Per project-wide instruction “Avoid repeated string literals”, introduce a single canonical constant (e.g. `const PARAM_BOOL_TRUE: &str = "true";`) and/or a helper `fn param_is_true(map, key) -> bool`.

## Non-blocking / hygiene
- There is an empty “branch placeholder” commit (`ebf3c7b`) in history. Consider dropping/squashing it before merge.

## Analog / consistency assessment
- Using PARAMETERS for flags matches the existing pattern already used for `signal`, `pipeline`, `stage`, counters, etc. The approach is consistent.
- The chosen param keys (`pause`/`confirm`) are consistent with other simple keys, but note `pause` also exists as a state label name (`state:pause`). It’s not ambiguous in parsing (label prefix vs param key), but it may be confusing for humans; consider documenting if needed.

## Suggested fix summary
1) Add cleanup of legacy `flag:*` labels (repo + issue) since backward compatibility is not required.
2) Replace repeated `"true"` literals with a constant/helper.