# Test Plan: `--select` flag for `task process`

## Summary of implementation changes

A single file was modified: `zbobr/src/commands.rs`.

- `TaskSubcommand::Process` gained a `select: bool` field (`#[arg(long)`) and a `#[arg(conflicts_with = "select")]` annotation on the positional `task` arg.
- The handler was updated to branch on `select`: when true, it lists all tasks, sorts by ID, calls `select_runnable_task`, and processes the winner (or exits with code 1 if none).
- When `select` is false, behavior is unchanged (`require_task_id` path).

## Coverage assessment

**Already well-covered:**
- `select_runnable_task` function: 7+ dedicated unit tests in `zbobr-dispatcher/src/cli.rs` covering priority selection, tie-breaking, exclusions (paused, done, ready-with-stack), empty input, and all-filtered cases.

**Not covered:**
- The Clap-level `conflicts_with` enforcement — that `task process 42 --select` is rejected at parse time. This was the primary correctness concern flagged across multiple review rounds.
- There are currently zero Clap parse tests in the entire `zbobr` crate.

## Tests proposed

### Group 1: Clap parse-level tests (in `zbobr/src/commands.rs`)

Three synchronous unit tests using `Cli::try_parse_from`:

1. **`task_process_select_flag_parses_without_task_id`** — `task process --select` → `select=true, task=None`
2. **`task_process_explicit_id_parses_without_select`** — `task process 42` → `task=Some(42), select=false`
3. **`task_process_select_and_task_id_together_is_rejected`** — `task process 42 --select` → `Err` (conflicts_with)

## Tests NOT proposed

- Snapshot tests of prompt text or default config literals — not applicable here.
- Handler behavior end-to-end (e.g., "actually processes the highest-priority task") — this requires a full async task backend; `select_runnable_task` itself is already comprehensively tested at the unit level, and the handler wiring is a thin delegation identical to the `task list --select` analog.
