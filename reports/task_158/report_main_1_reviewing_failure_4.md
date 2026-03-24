# Review Report for Task 158

## Summary

The implementation of task 158 is incomplete and does not meet the requirements specified in the plan. While the new label format for GitHub has been partially introduced, the required cleanup and refactoring of the `State` enum in `zbobr-api` has not been performed. The old string-based representations and constants still exist, and consumers outside of the GitHub backend have not been updated.

## Issues Found

### 1. Incomplete Refactoring of `State` Enum

The plan explicitly required:
> Remove `State::LABEL_DONE/PAUSE/READY/PENDING/RUNNING`, `ALL_LABEL_NAMES`, and `label_name()` from `zbobr-api/src/task.rs`. Define equivalent local constants in `zbobr-task-backend-github/src/github.rs`.

**Current Implementation:**
- `zbobr-api/src/task.rs` contains public constants `LABEL_DONE`, `LABEL_PAUSE`, etc., and the `label_name()` method.
- `zbobr-task-backend-github/src/github.rs` relies on these public constants (`State::LABEL_DONE`) instead of defining its own private ones.

This violates the architectural goal of decoupling the API from GitHub-specific label names.

### 2. Failure to Remove Legacy Trait Implementations

The plan required:
> Rewrite State Display/From/serde in `zbobr-api/src/task.rs`: (1) Remove `impl Display for State`. (2) Remove `PartialEq<&str>` and `PartialEq<String>`. (3) Remove `contains()` and `ends_with()` methods.

**Current Implementation:**
- `impl Display for State` still exists.
- `impl PartialEq<&str> for State` and `impl PartialEq<String> for State` still exist.
- `contains()` and `ends_with()` methods still exist on `State`.

### 3. Callers Not Updated

The plan listed specific files to update to use typed methods instead of string operations. These remain unchanged:

- **`zbobr-task-backend-fs/src/fs.rs`**:
  - Uses `task.state.to_string()` (line 95).
  - Uses comparison `if task.state == "DONE"` (line 645).
- **`zbobr-dispatcher/src/cleanup.rs`**:
  - Uses comparison `if task.state == "DONE"` (line 42).
- **`zbobr/src/commands.rs`**:
  - Uses `default_value = "READY"` (line 70).

### 4. Tests Likely Not Updated

Given that the API changes were not completed, the tests that were supposed to be updated (e.g., in `abstract_test_helpers.rs`, `test_helpers.rs`, `env.rs`) likely still rely on the old string-based behavior.

## Recommendations

1.  **Complete the API Refactor**: Remove the `Display`, `PartialEq<str/String>`, `contains`, and `ends_with` implementations from `State`. Remove `LABEL_*` constants from `State`.
2.  **Move Constants**: Define private constants (e.g., `const LABEL_DONE: &str = "done";`) inside `zbobr-task-backend-github/src/github.rs` and update the implementation there to use them.
3.  **Update Callers**: Refactor all usages of `task.state` in `fs.rs`, `cleanup.rs`, and command definitions to use the typed methods (`is_done()`, `is_pending()`, etc.) or the new `Debug` formatting where appropriate.
4.  **Fix Tests**: Update all tests that rely on string comparison for `State` to use the typed helper methods.
