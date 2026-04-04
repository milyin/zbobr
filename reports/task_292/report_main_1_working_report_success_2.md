# Fix: Clap-level exclusivity for `task process --select` vs positional `task`

## Change

In `zbobr/src/commands.rs`, the `Process` variant of `TaskSubcommand`:

**Before:**
```rust
Process {
    /// Task ID
    task: Option<u64>,
    /// Select the highest-priority ready task and process it; exits with code 1 if none
    #[arg(long)]
    select: bool,
},
```
Handler had: `if task.is_some() && select { anyhow::bail!("--select and a task ID are mutually exclusive"); }`

**After:**
```rust
Process {
    /// Task ID
    #[arg(conflicts_with = "select")]
    task: Option<u64>,
    /// Select the highest-priority ready task and process it; exits with code 1 if none
    #[arg(long)]
    select: bool,
},
```
Handler: manual `anyhow::bail!` check removed.

## Outcome
- Exclusivity is now enforced declaratively in the Clap parser layer, not at runtime in the handler.
- Invalid invocations (both a task ID and `--select`) are rejected during argument parsing with a proper Clap error message.
- Build passes successfully.