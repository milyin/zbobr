# Update separator.rs: CHECKLIST → CONTEXT

## Changes

**File:** `zbobr-task-backend-github/src/separator.rs`

### Constants
- Removed `CHECKLIST_SEPARATOR = "\n\n---CHECKLIST---\n"`
- Added `CONTEXT_SEPARATOR = "\n\n---CONTEXT---\n"`

### Functions updated
1. **`parse_description_full`** — signature changed from returning `(String, HashMap, Option<String>, Vec<ChecklistItem>)` to `Result<(String, HashMap, Option<String>, TaskContext)>`. Uses `context_format::parse_context` instead of `checklist_format::parse_grouped_checklist`. Errors propagated via `?`.

2. **`serialize_description_full`** — 4th parameter changed from `&[ChecklistItem]` to `&TaskContext`. Uses `context_format::serialize_context(ctx, &[], false)`. On parse failure in `original_description`, falls back to using it as-is.

3. **`merge_concurrent_description_updates`** — now returns `Result<String>` since `parse_description_full` is fallible. Compares `TaskContext` via `serde_json::to_string` (same approach as before with checklist).

### Imports updated
- Removed: `checklist_format::{parse_grouped_checklist, serialize_grouped_checklist}`, `task::ChecklistItem`
- Added: `anyhow::Result`, `context_format::{parse_context, serialize_context}`, `task::TaskContext`

### Tests rewritten
- `roundtrip_preserves_context` — verifies context records survive serialize/parse roundtrip
- `empty_context_not_serialized` — verifies empty context produces no CONTEXT section
- `roundtrip_preserves_error_section` — verifies all sections (PARAMETERS, ERROR, CONTEXT) coexist correctly
- `roundtrip_no_error_section` — verifies optional sections are omitted when empty
- `merge_preserves_non_conflicting_changes` — verifies concurrent error + context changes merge correctly
- `merge_our_change_wins_on_conflict` — verifies last-write-wins for context conflicts

### Note
The github backend (`github.rs`) has expected compilation errors since it still references `task.checklist` and the old function signatures. These will be fixed in the next checklist item (`update-backends`).

## Commit
`3517e05` on branch `zbobr_fix-163-context-structure`
