## Plan: Add `--force` flag to `init` command

### Context
The `init` command currently uses a safe `write_or_new()` function that creates `.new` files instead of overwriting existing files with different content. The task requires adding a `--force` flag that makes `init` overwrite destination files directly.

### Analog
The `Setup` command already has a `--force` flag defined with `#[arg(long, short = 'f')]` in `commands.rs`. The `init` implementation should follow the same pattern.

### Changes

1. **Add `force` field to `Init` variant** in `commands.rs` — add `#[arg(long, short = 'f')] force: bool` following the same pattern as `Setup`.

2. **Pass `force` through `main.rs`** — update the `Init` match arm to extract and pass the `force` flag to `init_workspace()`.

3. **Update `init.rs`** — accept `force` parameter in `init_workspace()` and pass it to `write_or_new()`. When `force` is true and a file exists with different content, overwrite it in place instead of writing to `.new`.

### Verification
- Run `cargo build` to ensure it compiles
- Run `cargo test` if tests exist
- Test manually: `zbobr init /tmp/test-workspace` (creates files), then `zbobr init /tmp/test-workspace` (creates `.new`), then `zbobr init --force /tmp/test-workspace` (overwrites in place)