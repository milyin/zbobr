## Summary

Addressed both issues raised in the review of the previous working session:

### 1. "true" literal violation
Added `const FLAG_VALUE_TRUE: &str = "true";` and replaced all occurrences of the hardcoded `"true"` string in:
- `issue_to_task`: comparison `s == "true"` → `s == FLAG_VALUE_TRUE`
- `task_to_string_params`: `"true".to_string()` → `FLAG_VALUE_TRUE.to_string()`
- Tests: same replacements

### 2. Old flag labels not cleaned up
Added `const FLAG_LABEL_PREFIX: &str = "flag:";` and updated `apply_state_change` to also strip labels starting with `FLAG_LABEL_PREFIX` (legacy `flag:pause`, `flag:confirm`) in addition to the existing `state:` label cleanup. This ensures existing GitHub issues with old-style flag labels are cleaned up transparently when a task is next saved.

### Verification
- `cargo build -p zbobr-task-backend-github` passes
- `cargo test -p zbobr-task-backend-github` passes (18 tests)
