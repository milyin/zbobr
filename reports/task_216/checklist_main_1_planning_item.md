In `github.rs`, the `issue_to_task` function currently reads `pause` and `confirm` by iterating over issue labels and calling `label_to_flag`. Replace this with reading the values from `params_map` using new parameter-name constants.

Add two constants near the top of the file (alongside existing `STATE_PREFIX` etc.):
- `PARAM_PAUSE` for the pause parameter key name ("pause")
- `PARAM_CONFIRM` for the confirm parameter key name ("confirm")

Then in `issue_to_task`, replace the two `.labels.iter().any(...)` blocks with params_map lookups using those constants (truthy when value is "true").