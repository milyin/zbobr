# Unit tests for `select_runnable_task`

**File:** `zbobr-dispatcher/src/cli.rs` — add `#[cfg(test)] mod tests` block

**Test helpers needed:**
- `make_workflow()` — creates a `Workflow::from_config(...)` with a single "main" pipeline containing one "working" stage with role "worker"
- `make_task(id, state, stage_count, pause, stack)` — constructs a `Task` with the given fields, defaults for the rest

**Tests:**

1. `selects_highest_stage_count` — two Ready tasks with stage_count 2 and 5; selected task has id matching stage_count=5
2. `deterministic_tie_break_on_equal_stage_count` — two Ready tasks with identical stage_count but different ids; result is always the same regardless of input order
3. `excludes_paused_tasks` — single Ready task with `pause: true`; returns None
4. `excludes_ready_with_nonempty_stack` — single Ready task with non-empty `stack` vec; returns None (matches loop Phase 1 normalization)
5. `excludes_done_tasks` — single Done task; returns None
6. `returns_none_when_empty` — empty slice; returns None
7. `returns_none_when_all_filtered` — mix of paused + done + ready-with-stack tasks; returns None

**Dependencies:** `Workflow::from_config`, `WorkflowConfig`, `PipelineConfig`, `StageDefinition`, `Task`, `State`, `StackEntry`, `Signal`, `Pipeline`, `Stage`, `TaskContext`