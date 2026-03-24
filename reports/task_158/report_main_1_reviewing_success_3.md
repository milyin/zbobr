# Review Report: Replace Milestones with Labels

## Findings

1.  **Correctness**: The core logic of replacing milestones with labels for task state (`state:`, `pipeline:`, `stage:`) was correctly implemented in `zbobr-task-backend-github`.
2.  **Requirements**:
    *   State representation requirements met.
    *   Legacy code removed.
    *   Display trait removed.
3.  **API Cleanliness (Addressed)**:
    *   The initial implementation left `SIGNAL_PREFIX` ("signal:") in `zbobr-dispatcher`, violating the requirement to move all prefix-related code to the GitHub backend.
    *   **Fix**: I refactored the `TaskBackend::setup` trait method to accept `&[Signal]` instead of `&[String]` (which were expected to be pre-formatted labels).
    *   I updated `zbobr-dispatcher` to pass `Signal` objects directly, removing its knowledge of the "signal:" prefix.
    *   I updated `zbobr-task-backend-github` to handle the prefixing internally when converting `Signal`s to labels.
    *   I implemented `Ord` for `Stage` and `Signal` to support sorting/deduplication of signals.

## Changes Verified

*   `zbobr-api/src/task.rs`: `State` enum updated, `Signal` implements `Ord`.
*   `zbobr-api/src/backend.rs`: `TaskBackend::setup` signature updated.
*   `zbobr-dispatcher/src/lib.rs`: Removes `SIGNAL_PREFIX`, constructs `Vec<Signal>`.
*   `zbobr-task-backend-github/src/github.rs`: Implements `setup` using `Signal`s, handles label prefixing.
*   `zbobr-task-backend-fs/src/fs.rs`: Updated signature.
*   `zbobr-dispatcher/src/task.rs`: Updated mock backend signature.
*   All tests passed.

## Conclusion

The task is complete and the code is clean and decoupled. The GitHub-specific label representation for signals is now confined to the GitHub backend.