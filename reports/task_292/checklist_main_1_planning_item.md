In `zbobr/src/commands.rs`, locate the `Process` subcommand variant. It currently has a `task: Option<u64>` field. Add a `#[arg(long)] select: bool` field alongside it.

Analog: the `List` variant already has a `select: bool` field — follow the exact same clap attribute pattern.

Constraints:
- The two args (`task` and `--select`) should be mutually exclusive — if both are provided, return an error with a clear message (similar to how `list` handles invalid states).
- No new CLI argument groups are needed; a simple runtime check in the handler is sufficient.