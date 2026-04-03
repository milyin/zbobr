The plan has been submitted for review. It covers four main changes:

1. **`TaskListEntry` struct** — lightweight type with id, stage_count, state, title for compact list output
2. **Shared priority selection function** — extracted from loop logic, reused by `--select`
3. **`task list` updates** — compact one-line format by default, `--json` for JSON array, `--select` to print highest-priority ready task id (exit 1 if none)
4. **`task show --json`** — full Task struct serialized as JSON

Waiting for user feedback on the plan.