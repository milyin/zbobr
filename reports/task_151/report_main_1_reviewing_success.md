# Code Review Report

## Summary
The changes successfully implement the requirement to synchronize "signal:*" labels during the setup command in the GitHub backend. The implementation ensures that only necessary signal labels (Go, Call, Return, ReturnFailure) exist, removing obsolete ones and creating missing ones, while respecting existing labels to avoid unnecessary API calls.

## Findings

### 1. Correctness and Completeness
- **Trait Update**: `TaskBackend::setup` signature was correctly updated to accept `signal_labels: &[String]`.
- **Label Computation**: `ZbobrDispatcher::setup` correctly computes the full set of required signal labels based on the workflow configuration (stages, pipelines) and static signals (`return`, `return_failure`). This matches the `Signal` enum variants.
- **Synchronization Logic**: The GitHub backend `setup` implementation correctly:
    - Identifies existing `signal:*` labels.
    - Deletes `signal:*` labels that are no longer in the required set.
    - Creates missing signal labels.
    - Updates existing signal labels only if `force` is true.
- **Backend Updates**: All `TaskBackend` implementations (GitHub, FS, Dummy, Task impl) were updated to match the new signature.

### 2. Code Quality and Consistency
- **Helper Method**: Added `delete_label` helper in `ZbobrTaskBackendGithubImpl`, consistent with existing `create_label` and `update_label`.
- **Error Handling**: Uses `anyhow::Result` and `retry_github` wrapper, consistent with the rest of the backend code.
- **Logging**: Appropriate `tracing::info!` logs added for label operations.
- **Style**: Variable naming and formatting follow project standards.

### 3. Analog Consistency
The approach extends the existing pattern used for "flag" labels but adds the necessary cleanup logic for dynamic signal labels. This is the correct approach as signal labels depend on the workflow configuration which can change.

## Conclusion
The implementation is correct, complete, and follows the project's architectural patterns. No issues found.
