# Test Plan: Timezone Fix (task-245)

## Summary
The implementation adds `timezone: Option<FixedOffsetTz>` to both task backend configs and applies `with_timezone()` to comment timestamps in `get_task_comments_internal` (GitHub) and `read_comments_structured` (FS). Two test groups are needed.

## Not Tested and Why
- **GitHub backend `get_task_comments_internal`**: conversion logic is embedded in an async function that calls the GitHub HTTP API. It's structurally identical to the FS backend case; the FS unit test provides sufficient behavioral coverage.
- **Default config values / prompt text**: not product-API contracts.

## Tests Required

### 1. `FixedOffsetTz::from_str` unit tests — `zbobr-api/src/task.rs`
The custom parser covers two format branches (`+HHMM`, `+HH:MM`), negative offsets, and several error cases. None are tested. Tests to add:
- Parses `+HHMM` (e.g. `+0300`)
- Parses `+HH:MM` (e.g. `+03:00`)
- Parses negative offset (e.g. `-05:30`)
- Parses UTC `+0000`
- Rejects empty string
- Rejects missing sign
- Rejects invalid digit content
- Serde roundtrip (serialize → deserialize → equal)

### 2. FS backend `read_comments_structured` timezone conversion — `zbobr-task-backend-fs/src/fs.rs`
New conditional branch has zero coverage. Tests to add:
- With `timezone: Some(+03:00)`: UTC noon timestamp shifts to 15:00 local
- With `timezone: None`: timestamp is returned unchanged

Add `tempfile` as `[dev-dependencies]` in `zbobr-task-backend-fs/Cargo.toml` if not already present.
