# Step 1: Define new data structures — Complete

## Changes

### zbobr-api/src/task.rs
Added new context types after the existing ChecklistItem definition:

1. **ContextRecordType** enum with serde rename_all snake_case:
   - `Checkbox(bool)` — checked/unchecked state
   - `Success` — success report
   - `Failure` — failure report  
   - `Comment` — a comment
   - `Question` — requires human input
   - Implements `Display` trait

2. **ContextRecord** struct:
   - `id: u64` — unique numeric id
   - `record_type: ContextRecordType`
   - `brief: String` — brief description
   - `report_link: Option<String>` — optional link to long description/report

3. **StageInfo** struct:
   - `pipeline: String`, `stage: String`
   - `tool: Option<Tool>`, `model: Option<Model>`
   - `prompt_link: Option<String>`
   - `timestamp: String`

4. **StageContext** struct:
   - `info: StageInfo`
   - `records: Vec<ContextRecord>`
   - `user_comment: Option<String>`

5. **TaskContext** struct with `Default` derive:
   - `stages: Vec<StageContext>`
   - Methods: `next_id()`, `find_record(id)`, `find_record_mut(id)`, `delete_record(id)`

6. **Task struct**: Added `context: TaskContext` field (with `#[serde(default)]`) alongside existing `checklist` field. The checklist field will be removed in step 9 after all references are migrated.

### zbobr-api/src/lib.rs
Added exports: `ContextRecord`, `ContextRecordType`, `StageContext`, `StageInfo`, `TaskContext`

## Tests added
- `task_context_default_is_empty` — default context has no stages, next_id returns 1
- `task_context_next_id` — correctly computes max id + 1 across multiple stages
- `task_context_find_record` — finds records by id across stages, returns stage index
- `task_context_find_record_mut` — mutable access to modify record types
- `task_context_delete_record` — removes records, returns false for missing ids
- `context_record_type_display` — Display impl for all variants

## Test results
All 23 tests pass (17 existing + 6 new).

## Commit
9fcc9c8 — "Add TaskContext data structures to zbobr-api"
