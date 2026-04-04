## Summary

Addressed the review feedback by reverting unrelated formatting-only changes from the task branch.

## Changes made

Restored 4 files to their `origin/main` state using `git checkout origin/main`:
- `zbobr-api/src/config.rs` — formatting-only changes (import order, line wrapping)
- `zbobr-api/src/config_tools.rs` — blank line removal
- `zbobr-dispatcher/src/cli.rs` — line wrapping change
- `zbobr/src/main.rs` — line wrapping change

## Branch state after fix

Only 2 task-relevant files differ from `origin/main`:
- `zbobr/src/commands.rs` — appends instance name to workspaces and repos_dir paths
- `zbobr-dispatcher/tests/mcp_integration/env.rs` — mirrors instance-scoped paths in test environment

## Verification

- `cargo test -p zbobr-api` — 126 passed
- `cargo test -p zbobr-dispatcher` — 0 passed, 8 ignored (all ignored are full GitHub backend tests requiring `--ignored` flag)
- `cargo test -p zbobr` — passed
- 3 pre-existing failures in `zbobr-task-backend-github` (CryptoProvider issue, unrelated to this task)
- `cargo fmt --check` reports pre-existing formatting issues in files on `origin/main` — not introduced by this branch