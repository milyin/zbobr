
## What
Update `serialize_description_full` in `zbobr-task-backend-github/src/separator.rs` to accept and forward comments so they appear interspersed in the CONTEXT section of the GitHub issue body.

## Why
Currently `serialize_description_full` always passes `&[]` (empty comments) to `serialize_context`, so no comments are shown in the task description. The task requires compact comment titles to appear alongside stage entries in the user-display context.

## Changes to `separator.rs`

### `serialize_description_full`
- Add parameter `comments: &[zbobr_api::task::Comment]` (after `context`).
- Change the internal call from `serialize_context(context, &[], false, report_url)` to `serialize_context(context, comments, false, report_url)`.
- Import `Comment` from `zbobr_api::task` if not already imported.

### `merge_concurrent_description_updates`
- This function merges structural edits (description, parameters, status, context) and does not need to display comments — it operates on parsed domain objects and re-serializes. Keep passing `&[]` in the final `serialize_description_full` call within this function.
- The function signature itself does NOT need a `comments` parameter.

### Tests in `separator.rs`
- Update all existing `serialize_description_full` call sites in tests to pass `&[]` as the new `comments` argument.
- Add a new test that passes a `Comment` with `report_name = Some(...)` and verifies the serialized output contains a compact comment line (the preview text and formatted date) and a `<!-- stage -->` marker before the stage.

## Analog
Follow the same pattern as the `report_url` parameter threading: optional/slice argument forwarded into `serialize_context`.
