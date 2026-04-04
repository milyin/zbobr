# Fix: Clap-level argument exclusivity for `task process`

## Problem
The review found that mutual exclusion between the positional `task` argument and `--select` flag in `task process` was enforced via a runtime `anyhow::bail!` check in the handler, rather than at the Clap parser level.

## Change Made
**File:** `zbobr/src/commands.rs`

1. Added `#[arg(conflicts_with = "select")]` to the `task` positional argument in the `Process` variant — this tells Clap to reject the combination during argument parsing.
2. Removed the manual runtime check (`if task.is_some() && select { anyhow::bail!(...) }`) from the handler.

## Result
- Invalid invocations (e.g., `zbobr task process 42 --select`) are now rejected by Clap during parsing with a proper CLI error, consistent with the rest of the CLI surface.
- `cargo build` succeeds with no warnings related to these changes.