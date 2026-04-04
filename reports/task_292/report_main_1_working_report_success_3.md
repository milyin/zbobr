## What was done

Added `#[arg(conflicts_with = "select")]` to the `task: Option<u64>` field in `TaskSubcommand::Process` in `zbobr/src/commands.rs`.

Removed the manual runtime check:
```rust
if task.is_some() && select {
    anyhow::bail!("--select and a task ID are mutually exclusive");
}
```

Clap now declaratively enforces the exclusivity at parse time, producing a standard Clap error with help output when `task process 123 --select` is invoked — consistent with the pattern used for other mutually exclusive args in the codebase.

## Commit

`dc3ed7fd` — fix: enforce task vs --select exclusivity in Clap parser layer

## Verification

`cargo build -p zbobr` succeeds with no warnings or errors.