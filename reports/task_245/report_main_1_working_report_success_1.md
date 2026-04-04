# Fix: Use FixedOffsetTz type for backend timezone field

## Problem
The review rejected the previous implementation because it used `Option<i32>` (raw offset seconds) instead of the existing `FixedOffsetTz` domain type, and required runtime reconstruction of `chrono::FixedOffset` from raw seconds with fallback branches.

## Changes Made

### Config files
- **`zbobr-task-backend-github/src/config.rs`**: Renamed `timezone_offset_seconds: Option<i32>` → `timezone: Option<zbobr_api::task::FixedOffsetTz>`
- **`zbobr-task-backend-fs/src/config.rs`**: Same change

### Backend implementations  
- **`zbobr-task-backend-github/src/github.rs`**: Simplified `get_task_comments_internal()` to use `self.backend_config.timezone` directly with `&*tz` deref. Removed `FixedOffset::east_opt()` reconstruction and fallback.
- **`zbobr-task-backend-fs/src/fs.rs`**: Simplified `read_comments_structured()` similarly — removed nested `if let Some(tz) = FixedOffset::east_opt(secs)` and silent no-op fallback.

### Injection site
- **`zbobr/src/commands.rs`**: Changed from `tasks_config.timezone_offset_seconds = Some(dispatcher_config.fixed_offset().local_minus_utc())` to `tasks_config.timezone = dispatcher_config.timezone` — direct pass-through of the `Option<FixedOffsetTz>` without lossy conversion.

### Test/init sites
- **`zbobr-dispatcher/tests/mcp_integration/env.rs`**: Updated field name to `timezone: None`
- **`zbobr-task-backend-github/src/github.rs`** (test helper `make_config()`): Same
- **`zbobr/src/init.rs`**: Same

## Verification
`cargo check --tests` passes cleanly.