## Fix Summary

Applied `cargo fmt --all` to resolve formatting diffs reported in the linting stage. Verified both `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass cleanly.

### Files changed
- `zbobr/src/commands.rs` — reformatted the instance-path-join line from the task's feature change
- `zbobr/src/main.rs` — pre-existing formatting fix
- `zbobr-api/src/config.rs` — pre-existing formatting fixes (long assert lines, method chains)
- `zbobr-api/src/config_tools.rs` — removed trailing blank line
- `zbobr-dispatcher/src/cli.rs` — reformatted long conditional expression

### Commit
`cb084fcc` — `style: apply cargo fmt formatting fixes`

### Verification
- `cargo fmt --all -- --check` — passes (no diffs)
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passes (no warnings)