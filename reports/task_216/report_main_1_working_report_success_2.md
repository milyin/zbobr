# Fix: Legacy flag: label cleanup on every save

## Problem
The reviewer found that `apply_state_change` (which removes legacy `flag:pause` and `flag:confirm` labels) was only called when the task state changed. Tasks saved without a state transition would retain their old `flag:` labels indefinitely.

## Fix
Removed the `if task.state != original_state` guard in `modify_task_internal` so `apply_state_change` is always called on every save. This ensures legacy `flag:` labels are removed unconditionally.

Also removed the now-unused `original_state` variable to eliminate the compiler warning.

## Commit
`3bd936a` — `fix(#216): always apply state change to remove legacy flag: labels on every save`
