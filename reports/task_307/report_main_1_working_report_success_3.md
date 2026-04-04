## Fix: Move `tempfile` to `[dev-dependencies]`

### Problem
The review (ctx_rec_19) found that `tempfile` was added under `[dependencies]` in `zbobr/Cargo.toml`, but it is only used in test code (`#[tokio::test]` cases in `zbobr/src/init.rs`). This unnecessarily added a test helper to the runtime dependency set, inconsistent with the workspace convention.

### Fix
Moved `tempfile = { workspace = true }` from `[dependencies]` to a new `[dev-dependencies]` section in `zbobr/Cargo.toml`, matching the pattern used by other workspace crates (`zbobr-task-backend-fs`, `zbobr-dispatcher`, `zbobr-repo-backend-fs`).

### Verification
- `cargo build -p zbobr` — succeeds
- `cargo test -p zbobr` — all 18 tests pass (including the 4 `write_or_new` tests that use `tempfile`)

### Commit
`50535a7c` — `fix: move tempfile to dev-dependencies in zbobr/Cargo.toml`