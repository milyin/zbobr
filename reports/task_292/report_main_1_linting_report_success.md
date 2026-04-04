Commands run:
- cargo fmt --all -- --check (initial)
- cargo fmt --all (applied fixes)
- git add -A && git commit -F /tmp/commit_msg (committed auto-fixes)
- cargo fmt --all -- --check (re-check)
- cargo clippy --all-targets --all-features (lint)

Result:
- cargo fmt modified 2 files and commit was created on branch zbobr_fix-292-add-select-flag-for-task-process (commit 3e189d89)
- Files changed: zbobr-api/src/context/mod.rs, zbobr/src/commands.rs
- cargo fmt check passed after fixes
- cargo clippy completed with no warnings/errors

Notes:
- Only automated formatting fixes were applied. No manual code changes were made.
- If CI uses additional linters not detected in this repo, run them separately.
