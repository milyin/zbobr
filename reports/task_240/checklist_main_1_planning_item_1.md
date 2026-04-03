## What

Modify the `TaskSubcommand::List` variant in `zbobr/src/commands.rs` (around line 62) and its match arm (around line 310):

### CLI definition changes
- Add `--json` flag (bool, default false) to the `List` variant
- Add `--select` flag (bool, default false) to the `List` variant
- Keep the existing optional `state` filter parameter

### Match arm behavior changes

**Default (no flags):** Change from calling `print_task()` per task to printing a compact one-line-per-task format:
- Format: `{id}\t{stage_count}\t{state}\t{title}` — one line per task
- Use `TaskListEntry` to map each task before display

**--json flag:** Serialize `Vec<TaskListEntry>` as a JSON array to stdout using `serde_json::to_string_pretty` (or `to_string`). The state filter should still apply before JSON serialization.

**--select flag:** Use the shared `select_ready_task` function on the task list. If a ready task is found, print just its id to stdout and exit with code 0. If no ready task is found, print nothing and exit with code 1 (use `std::process::exit(1)` or return an appropriate error).

## Why
- The compact format makes `task list` useful for quick scanning (the current verbose multi-line output is better suited for `task show`)
- JSON output enables scripting and integration with other tools
- `--select` enables automation: scripts can pick the next task to process without parsing human-readable output

## Analog
Follow the existing pattern of how `List` fetches and filters tasks (lines 310-336), just change the output formatting and add the new flag branches.