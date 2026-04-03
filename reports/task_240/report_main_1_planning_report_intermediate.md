# Plan: Update task list, add JSON output

## Context
The `task list` command currently outputs verbose multi-line task details (same as `task show`). It needs to be changed to a compact one-line-per-task format showing only key fields. Additionally, JSON output support is needed for both `task list` and `task show`, and a `--select` flag to pick the highest-priority ready task (reusing the same priority logic as the `loop` command).

## Closest Analog
No existing `--json` flag pattern exists in the CLI. The Task struct already derives `serde::Serialize`, so JSON serialization is straightforward. The priority selection logic exists inline in `run_manager_loop` at `zbobr-dispatcher/src/cli.rs:1082`.

## Changes

### 1. Create `TaskListEntry` struct in `zbobr-dispatcher/src/cli.rs`
- A lightweight struct with fields: `id`, `stage_count`, `state`, `title` (the task description field is too long for one-line display; title serves as the short description)
- Derive `Serialize` for JSON output
- Implement `From<&Task>` or a constructor

### 2. Extract ready-task selection function in `zbobr-dispatcher/src/cli.rs`
- Create a function like `select_ready_task(tasks: &[Task]) -> Option<&Task>` that:
  - Filters to tasks in Ready state (not Done, not Pause, not paused)
  - Sorts by `stage_count` descending (highest priority first)
  - Returns the first match
- Refactor the loop in `run_manager_loop` to use this function for its task ordering/selection logic (or at minimum share the sorting/priority comparison)
- Export from `zbobr-dispatcher/src/lib.rs`

### 3. Update `task list` command definition in `zbobr/src/commands.rs`
- Add `--json` flag (`bool`) to `TaskSubcommand::List`
- Add `--select` flag (`bool`) to `TaskSubcommand::List`
- Update the `List` match arm:
  - Default (no flags): print compact one-line per task — `{id}\t{stage_count}\t{state}\t{title}`
  - `--json`: serialize `Vec<TaskListEntry>` as JSON array to stdout
  - `--select`: use the shared selection function, print just the task id, exit with code 1 if no ready task found

### 4. Add `--json` flag to `task show` in `zbobr/src/commands.rs`
- Add `--json` flag (`bool`) to `TaskSubcommand::Show`
- When set, serialize the full `Task` struct as JSON to stdout instead of calling `print_task`

## Verification
- `zbobr task list` — compact one-line output
- `zbobr task list --json` — JSON array of list entries
- `zbobr task list --select` — prints id of highest-priority ready task or exits 1
- `zbobr task show <id> --json` — full task JSON
- `cargo build` and `cargo test` pass