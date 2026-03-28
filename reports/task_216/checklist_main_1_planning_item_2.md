In `github.rs`:

1. Remove the `apply_flag_change` async method entirely (it adds/removes GitHub labels for `flag:pause` and `flag:confirm`).
2. In the `save_task` method, remove the conditional block that calls `apply_flag_change` when `pause` or `confirm` changed (currently around line 913-915). Flags are now persisted via the body params, so no separate label API calls are needed.