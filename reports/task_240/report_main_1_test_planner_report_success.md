# Test Plan: task list / task show CLI changes

## Summary

The implementation adds `TaskListEntry`, `select_runnable_task`, `task_priority`, and CLI flags (`--json`, `--select` for `task list`; `--json` for `task show`). The shared `select_runnable_task` caused 5 review round-trips due to subtle filtering/priority bugs. None of the new functions have test coverage. Two test groups are proposed.

## Test Group 1: `select_runnable_task` (7 tests)

Target: `zbobr-dispatcher/src/cli.rs`

| # | Test name | Behavior validated |
|---|-----------|-------------------|
| 1 | `selects_highest_stage_count` | Picks task with highest stage_count among runnable |
| 2 | `deterministic_tie_break` | Same stage_count → deterministic winner via task_id |
| 3 | `excludes_paused_tasks` | `pause: true` → filtered out |
| 4 | `excludes_ready_with_stack` | READY + non-empty stack → filtered (Phase 1 semantics) |
| 5 | `excludes_done_tasks` | Done state → not runnable |
| 6 | `returns_none_empty_input` | Empty slice → None |
| 7 | `returns_none_all_filtered` | All tasks disqualified → None |

Uses `Workflow::from_config(...)` with a minimal single-pipeline config, following patterns from `zbobr-dispatcher/src/workflow.rs` tests.

## Test Group 2: `TaskListEntry` (2 tests)

Target: `zbobr-dispatcher/src/cli.rs`

| # | Test name | Behavior validated |
|---|-----------|-------------------|
| 1 | `from_task_projects_fields` | `From<&Task>` copies id, stage_count, state, title |
| 2 | `json_has_expected_keys` | Serialized JSON has exactly the 4 expected keys (--json contract) |

## Excluded from testing

- `task_priority` — trivial one-liner, indirectly covered
- CLI flag parsing — covered by clap derive macros
- Prompt/config text content — not behavioral contracts
- Loop Phase 2 integration — requires async dispatcher; shared logic tested via selector unit tests
