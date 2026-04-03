# Fix: select_runnable_task READY-with-stack exclusion

## Problem
`task list --select` could return a task ID that the loop would not actually run in Phase 2 of the current cycle. Specifically, tasks in `READY` state with a non-empty stack were included in `select_runnable_task`'s candidates and resolved via `workflow.resolve_next_action()` directly on their raw state. This produced a wrong answer because `pipeline_from_state(State::Ready)` returns `None`, falling back to the default pipeline instead of the saved stack pipeline.

The loop, by contrast, handles these tasks in Phase 1 (`apply_ready_from_state`: pop stack, set `Pending(saved_pipeline)`, set saved signal, then `continue`) — they are **never** added to Phase 2's `runstage_candidates`.

## Fix
Added a guard in `select_runnable_task` (zbobr-dispatcher/src/cli.rs):

```rust
let ready_with_stack = t.state.is_ready() && !t.stack.is_empty();
!t.pause
    && !ready_with_stack
    && matches!(...)
```

READY-with-stack tasks are now excluded from `--select`, making the helper a true single source of truth shared by both `--select` and the loop's Phase 2.

## Verification
- `cargo build -p zbobr-dispatcher` passes with no errors or warnings
- Committed: `fix: exclude READY-with-stack tasks from select_runnable_task`