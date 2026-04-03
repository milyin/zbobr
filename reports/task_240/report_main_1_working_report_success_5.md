# Fix: task show --json now routes through backend-aware path

## Problem
`TaskSubcommand::Show { id: None, .. }` was listed in `needs_backends()` as not requiring backends, so `zbobr task show` and `zbobr task show --json` (without an ID) were routed to `run_without_backends()`, which always printed the sample task. The real backend-aware code path (fetching and serializing `Vec<Task>`) was unreachable.

## Fix
Removed `TaskSubcommand::Show { id: None, .. }` from the `needs_backends()` exclusion list and removed its corresponding match arm in `run_without_backends()`. Now `task show` without an ID always goes through `run_task_subcommand()` which fetches real tasks from the backend.

## Files changed
- `zbobr/src/commands.rs`: removed Show{id:None} from needs_backends(), removed sample-task arm from run_without_backends()

## Verification
`cargo build -p zbobr` succeeds with no errors or warnings.