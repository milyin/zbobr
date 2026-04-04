## Linting Fix Summary

### Problem
`cargo fmt --all -- --check` reported formatting diffs in files changed by our branch (`zbobr/src/init.rs` and `zbobr/src/main.rs`), along with pre-existing formatting issues in other crates.

### Changes Made
- Ran `cargo fmt` and applied formatting fixes only to files modified by our branch:
  - `zbobr/src/init.rs`: reformatted `assert_eq!` macro call to multi-line style
  - `zbobr/src/main.rs`: reformatted `resolve_config_location` call to single line

- Pre-existing formatting issues in other crates (`zbobr-api`, `zbobr-dispatcher`) were left untouched as they are outside scope.

### Verification
- `cargo fmt --all -- --check`: no diffs in our files
- `cargo clippy -p zbobr --all-targets --all-features`: clean, no warnings
- `cargo test -p zbobr -- write_or_new`: all 4 tests pass

### Commit
`73a9c89a` — style: apply cargo fmt to init.rs and main.rs