## Summary

The plan is to mirror the `task list --select` pattern in `task process`. The analog is the existing `List` handler in `zbobr/src/commands.rs` which uses `select_runnable_task`.

### Key design decisions:
- Reuse `select_runnable_task` — already imported, no new utilities needed
- Mutual exclusivity of `--task` and `--select` enforced at runtime with a clear error
- Exit code 1 when `--select` finds no runnable task, matching `list --select` behavior
- No behavior change to existing `task process <id>` path

### Checklist items:
1. Add `select: bool` field to `Process` variant
2. Update handler to branch on `select` vs explicit task ID