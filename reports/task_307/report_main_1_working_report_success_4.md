## Linting Fix

### Problem
`cargo fmt --all -- --check` reported formatting diffs in three files:
- `zbobr-api/src/config.rs` (multiple locations: import ordering, method chain formatting, assert macro formatting, struct literal formatting)
- `zbobr-api/src/config_tools.rs`
- `zbobr-dispatcher/src/cli.rs`

These were **pre-existing formatting issues** in files not touched by our `init --force` feature branch. The workspace-wide lint check caught them.

### Fix
Ran `cargo fmt --all` to apply rustfmt to all affected files.

### Verification
- `cargo fmt --all -- --check` exits with code 0 (no diffs)
- `cargo clippy --workspace --all-targets -- -D warnings` exits with code 0 (no warnings)

### Commit
`5694d560` — `style: apply cargo fmt to pre-existing formatting issues`