## What

Modify the `TaskSubcommand::Show` variant in `zbobr/src/commands.rs` (around line 74) and its match arm (around line 337):

### CLI definition changes
- Add `--json` flag (bool, default false) to the `Show` variant

### Match arm behavior changes
- When `--json` is set and an ID is provided: serialize the full `Task` struct as JSON to stdout using `serde_json::to_string_pretty`. The Task struct already derives `Serialize`, so this is straightforward.
- When `--json` is set but no ID: could either error or serialize all tasks as JSON array — follow the existing behavior where no-ID falls through to listing all tasks, but serialize as JSON instead of calling print_task.

## Why
- JSON output for `task show` gives full task details in a machine-readable format, useful for scripting and debugging.

## Analog
Follow the existing `Show` match arm pattern — same task fetching logic, just branch on the `--json` flag for output format.