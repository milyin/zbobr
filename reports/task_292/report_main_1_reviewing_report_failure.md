Review result: changes are narrowly scoped to `zbobr/src/commands.rs` and mostly follow the planned analog (`task list --select`). Reusing `select_runnable_task` and preserving the exit-code-1 behavior for the no-match case are both consistent with the existing pattern. I did not find unrelated file changes.

Blocking finding:

1. Argument exclusivity is enforced in the wrong layer.
   - Location: `zbobr/src/commands.rs:130-136`, `zbobr/src/commands.rs:433-450`
   - The new `Process` variant adds `select: bool`, but the mutual exclusion between positional `task` and `--select` is enforced manually with `anyhow::bail!("--select and a task ID are mutually exclusive")` inside the handler.
   - Since this is a CLI shape/contract issue, it should be expressed declaratively in Clap on the arguments themselves (for example with `conflicts_with` / an arg group), so invalid invocations are rejected during parsing instead of after dispatch begins.
   - Why this matters:
     - It strengthens validation in the parser rather than relying on a runtime branch.
     - It keeps help/error behavior consistent with the rest of the CLI surface.
     - It avoids the risk of future handler refactors accidentally weakening or bypassing the exclusivity check.
   - This is the main inconsistency with the intended quality bar: the behavior works, but the contract is not encoded where the type/CLI system can enforce it.

Analog consistency assessment:
- Good: the implementation correctly mirrors the `task list --select` selection logic by listing tasks, snapshotting them, sorting by ID, and calling `select_runnable_task` before acting.
- Deviation: unlike a parser-level analog, the new command-specific exclusivity rule is not represented in the CLI definition.

Checklist assessment:
- The checked items correspond to the implemented work and there are no remaining unchecked checklist items in the provided context.

Overall:
- Functionally close and appropriately scoped, but I recommend fixing the parser-level validation before accepting the change.