# Test Planning Analysis

## Changes Reviewed
- `zbobr-task-backend-fs/src/fs.rs`: `read_comments_structured` now converts comment timestamps via `with_timezone` when `config.timezone` is `Some(tz)`
- `zbobr-task-backend-github/src/github.rs`: `get_task_comments_internal` applies same conversion inline
- Both backends gained `timezone: Option<FixedOffsetTz>` config fields
- `zbobr/src/commands.rs`: injects `dispatcher_config.timezone` into backend at construction

## Existing Test Coverage
- `flag_tests` in github.rs: tests issue parsing only; no comment-related tests
- No unit/integration tests in `zbobr-task-backend-fs/` (the file `tests/task_crud.rs` is entirely commented out)
- `FixedOffsetTz` in `zbobr-api/src/task.rs` has **no tests** (pre-existing gap, now more relevant)

## Test Gaps

### 1. `FixedOffsetTz` parser
The custom `from_str` implementation handles multiple format paths (+HHMM, +HH:MM) and several error cases. No tests exist. This type is now the primary carrier for the timezone feature.

### 2. FS backend comment timestamp conversion
`read_comments_structured` has a new conditional branch. It's testable with a temp dir + written YAML fixtures. The behavior contract: with `timezone: Some(tz)` → timestamps shifted; with `timezone: None` → timestamps unchanged.

### 3. GitHub backend comment timestamp conversion
Same logic but embedded in an async HTTP-calling function. A true unit test would require mock HTTP. The logic is structurally identical to the FS case, so FS coverage provides high confidence. Skipping a dedicated GitHub unit test unless a lightweight mock approach is already established in the codebase.

## Tests NOT Needed
- Snapshot tests of timezone offset formatting (e.g., "+0300" display) — not a product API contract
- Tests asserting default config literal values (`timezone: None`)
