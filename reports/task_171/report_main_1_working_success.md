# Session Report: ERROR Section Implementation (Items 1, 2, 4)

## Changes Made

### 1. `zbobr-task-backend-github/src/separator.rs` (item: error-section-separator)
- Added `ERROR_SEPARATOR` constant (`\n\n---ERROR---\n`)
- Updated `parse_description_full` to return 4-tuple `(String, HashMap, Option<String>, Vec<ChecklistItem>)`, parsing ERROR section between PARAMETERS and CHECKLIST
- Updated `serialize_description_full` to accept `error: &Option<String>` parameter and serialize it
- Updated `merge_concurrent_description_updates` to handle error section merging
- Updated existing tests for new 4-tuple signatures
- Added 2 new tests: `roundtrip_preserves_error_section` and `roundtrip_no_error_section`

### 2. `zbobr-api/src/task.rs` (item: task-struct-error)
- Added `pub error: Option<String>` field to `Task` struct with `#[serde(default, skip_serializing_if = "Option::is_none")]`
- Removed `Error` variant from `HistoryRecordType` enum
- Updated `classify_comment` to map `[report_error]`/`[stop_with_error]` to `HistoryRecordType::Other`

### 3. `zbobr-task-backend-github/src/github.rs` (item: github-backend-update)
- Updated `issue_to_task` to destructure 4-tuple and set `task.error`
- Updated all 4 `serialize_description_full` call sites to pass `&task.error`/`&None`

### 4. All other Task construction sites (compilation fixes)
- Added `error: None` to Task constructors in: `fs.rs`, `commands.rs`, `prompts.rs`, `task.rs`, `workflow.rs`

## Test Results
- All 4 separator tests pass
- `cargo check` passes for the full workspace

## Remaining Items
- backend-set-error, fs-backend-update, role-session-set-error, rewrite-stop-with-error, update-tests