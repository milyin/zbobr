# Completed: prefix-constants + replace-signal-flag-literals

## Changes

### zbobr-api/src/task.rs
- Added 5 module-level `pub const` prefix constants: `STATE_PREFIX`, `PIPELINE_PREFIX`, `STAGE_PREFIX`, `SIGNAL_PREFIX`, `FLAG_PREFIX`

### zbobr-api/src/lib.rs
- Exported all 5 prefix constants from the crate root

### zbobr-task-backend-github/src/github.rs
- Removed duplicate `STATE_PREFIX`, `PIPELINE_PREFIX`, `STAGE_PREFIX` constants from `impl GithubBackend`
- Imported all 5 constants from `zbobr_api`
- Replaced all `Self::STATE_PREFIX` / `Self::PIPELINE_PREFIX` / `Self::STAGE_PREFIX` with imported constants
- Replaced hardcoded `"signal:"` in `signal_to_label`, `label_to_signal`, signal label filtering, and description formatting (5 places)
- Replaced hardcoded `"flag:"` in `flag_to_label`, `label_to_flag` (2 places)
- Updated test to use `format!("{FLAG_PREFIX}confirm")` instead of `"flag:confirm"`

### zbobr-dispatcher/src/lib.rs
- Imported `SIGNAL_PREFIX` from `zbobr_api`
- Replaced 4 hardcoded `"signal:"` literals in signal label construction

## Verification
- `cargo check` — clean compilation
- `cargo test` — all tests pass