## What

After all changes, run `cargo test -p zbobr-api` and `cargo build --tests` to verify:

1. All 11 zbobr-api compilation errors are resolved
2. All 2 zbobr-dispatcher integration test compilation errors are resolved
3. All existing tests pass
4. The new/updated validation test correctly rejects roles without a tool

Also run `cargo clippy` to ensure no new warnings are introduced.