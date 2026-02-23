# Task Transitions

This document describes the **current** task-stage transitions implemented in code and is structured so new stages/signals can be added with minimal edits.

## Source of truth

- `zbobr-dispatcher/src/task.rs`
  - `Signal::target_stage`
  - `Stage` and `Signal` enums
- `zbobr/src/main.rs`
  - `run_role_session` (role completion signal policy)
  - manager loop pending-signal transition logic

## 1) Signal -> Target Stage

Edit this table when changing `Signal::target_stage`.

| Signal | Target stage |
| --- | --- |
| `stop` | `PENDING` |
| `done` | `PENDING` |
| `go_ask` | `PENDING` |
| `go_merge` | `GO_MERGING` |
| `go_review` | `GO_REVIEWING` |
| `go_work` | `GO_WORKING` |
| `go_plan` | `GO_PLANNING` |

## 2) Role completion -> Follow-up signal

Edit this table when changing post-session policy in `run_role_session`.

| Role | Follow-up signal rule |
| --- | --- |
| `preparator` | always `go_plan` |
| `planner` | no forced follow-up signal |
| `worker` | `go_work` if checklist has unchecked items, else `go_review` |
| `reviewer` | `go_work` if checklist has unchecked items, else `done` |
| `merger` | always `go_work` |

## 3) Stage processing policy

This reflects how processing works today:

1. Pending + signal processing:
   - If a task is in `PENDING` and has a signal with target stage != `PENDING`, move to that target stage.
2. Go-stage execution:
   - `GO_PREPARATION` -> run preparator session
   - `GO_PLANNING` -> run planner session
   - `GO_WORKING` -> run worker session
   - `GO_REVIEWING` -> run reviewer session
   - `GO_MERGING` -> run merger session
3. Active stages (`PREPARATION`, `PLANNING`, `WORKING`, `REVIEWING`, `MERGING`) are session stages; after session completion, tasks return to `PENDING` and follow-up signal is applied per section 2.

## 4) Update checklist for transition changes

When changing transitions, update all of the following in one PR:

- [ ] `Signal::target_stage` mapping in `zbobr-dispatcher/src/task.rs`
- [ ] role completion signal policy in `zbobr/src/main.rs`
- [ ] this document tables (sections 1 and 2)
- [ ] integration tests under `zbobr/tests/` that assert `Signal:` and `Stage:` behavior
