The plan is ready for review. Here's a summary of the proposed approach:

## Plan Summary

### Part 1: Simplify for-prompt context rendering
**File**: `zbobr-api/src/context/mod.rs`

Add `for_prompt: bool` to `MdRecord`, `MdStage`, `MdCompactComment`, and `MdContext`. Thread it through the existing conversion chain. Modify Display impls for `for_prompt=true`:
- **Stage header**: `- planning` (stage name only, no instance/pipeline/run_id/tool/model/timestamp)
- **Records**: `[ctx_rec_N]` plain text (no `<sub>` HTML tags, no URLs)
- **Comments**: `- user {name}: {text}` (no timestamp, no link)
- **Empty stages**: filtered out entirely

The `for_prompt=false` path (storage/display) is unchanged.

### Part 2: Add `get_ctx_rec` MCP tool
Follows the `delete_ctx_rec` pattern across 5 files:
- `config_tools.rs` — add `GetCtxRec` enum variant
- `common.rs` — add `GetCtxRecParam`  
- `task.rs` — add `get_context_record_content()` session method (finds record, reads linked file if present)
- `traits.rs` — add `get_ctx_rec_impl()`
- `unified.rs` — wire up as `#[tool]`