## Checklist item: create-context-format

### What was done
Created `zbobr-api/src/context_format.rs` implementing markdown serialization and parsing for `TaskContext`.

### Functions implemented
1. **`serialize_context(ctx: &TaskContext, comments: &[Comment], for_prompt: bool) -> String`**
   - Renders stage headers: `<!-- Stage: {pipeline} #{run_id} {stage} [{timestamp}] tool=... model=... prompt=... -->`
   - `for_prompt=true` omits prompt links from headers
   - Record lines with type prefixes: `- [ ]`/`- [x]` (Checkbox), `✅` (Success), `❌` (Failure), `💬` (Comment), `❓` (Question)
   - Report links as `[report](url)`, record IDs as `<sub>[ctx_rec_{id}]</sub>`
   - User comments interspersed by timestamp as blockquotes

2. **`parse_context(text: &str) -> Result<TaskContext>`**
   - Parses stage headers and record lines back into `TaskContext`
   - Ignores blockquote lines (user comments)
   - Returns `Err` on any parse failure (missing ID markers, records before stage headers, etc.)

### Tests (10 new, 27 total in zbobr-api)
- serialize_basic, serialize_for_prompt_omits_prompt_link
- parse_basic, parse_ignores_blockquote_comments
- roundtrip_preserves_data, roundtrip_for_prompt_loses_prompt_link
- parse_error_on_record_before_stage, parse_error_on_missing_id
- serialize_with_interspersed_comments, empty_context

### Notes
- `checklist_format.rs` kept alongside `context_format.rs` in lib.rs — downstream crates (separator.rs, fs.rs, prompts.rs) still reference it. Will be removed in later steps.
- Pre-existing build error in github backend (missing `context` field) is unrelated — will be fixed in `update-backends` step.

### Commit
`b19ba59` on branch `zbobr_fix-163-context-structure`
