Actions performed:
- Inspected repository for formatting/linting setup (no .github workflows present; Rust workspace detected in Cargo.toml).
- Ran: cargo fmt --all (applied formatting)
- Ran: cargo clippy --fix --all-targets --all-features --allow-dirty (applied clippy auto-fixes)
- Committed auto-fix changes (commit d5cf56b9) with message 'chore: fix formatting and linting' and Co-authored-by trailer.
- Re-ran checks: cargo fmt --all -- --check (passed), cargo clippy --all-targets --all-features -- -D warnings (passed).

Files changed in commit d5cf56b9:
- zbobr-api/src/context/mod.rs
- zbobr/src/commands.rs
- zbobr/src/init.rs

Summary: All formatting and linting checks now pass. Auto-fixes were applied and committed. No remaining lint warnings.