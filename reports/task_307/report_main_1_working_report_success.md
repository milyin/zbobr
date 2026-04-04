# Implementation: Add `--force` flag to `init` command

## Changes

### 1. `zbobr/src/commands.rs`
- Added `#[arg(long, short = 'f')] force: bool` field to the `Init` variant, following the same pattern as the `Setup` command.

### 2. `zbobr/src/main.rs`
- Updated the `Command::Init` match arm to destructure the new `force` field and pass it to `init::init_workspace()`.

### 3. `zbobr/src/init.rs`
- Added `force: bool` parameter to `init_workspace()`.
- Passed `force` to both `write_or_new()` call sites (prompt files and config file).
- Updated `write_or_new()` to accept `force: bool`. When `force` is true and a file exists with different content, it overwrites in place (printing `"  overwrote {path}"`) instead of writing to a `.new` path.

## Commits
1. `b47176ac` — init: add --force flag to Init command variant
2. `5eaa1299` — init: pass force flag from CLI to init_workspace()
3. `67ffdad1` — init: accept force param in init_workspace and write_or_new

## Verification
- `cargo build` — success
- `cargo test` — 14/14 zbobr crate tests pass. 3 pre-existing failures in `zbobr-task-backend-github` (CryptoProvider configuration issue, unrelated to this change).
