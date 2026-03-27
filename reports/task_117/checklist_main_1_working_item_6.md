## Problem
In `cleanup_legacy_token_config()` (github.rs:294), when `git(bare_dir, &["config", "--unset", key])` fails, the error `e` from `git()` contains the full args including the legacy key which has the token embedded (e.g. `url.https://x-access-token:TOKEN@github.com/.insteadOf`). Even though `redacted_key` is used in the log format, `e` still leaks the token.

## Fix
Use `tokio::process::Command` directly instead of the `git()` helper. Only log the redacted key and exit code, suppressing any error output that could contain the token.