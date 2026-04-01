# Plan Finalized

## Approach

Two coordinated changes:

### 1. Simplified for-prompt context rendering (`zbobr-api/src/context/mod.rs`)

Add `for_prompt: bool` to `MdRecord`, `MdStage`, `MdCompactComment`, and `MdContext`. Thread the flag through `from_task_context → from_stage_context → from_context_record`. Modify Display impls:
- `MdStage`: render `- {stage_name}` only (no metadata), skip stages with zero records
- `MdRecord`: render `[ctx_rec_N]` plain text (no `<sub>` HTML, no URL)
- `MdCompactComment`: render `- user {username}: {text}` (no timestamp, no link)

The `for_prompt=false` path (used for storage/display) is entirely unchanged.

### 2. `get_ctx_rec` MCP tool

Four-file change following the `delete_ctx_rec` pattern exactly:
- `config_tools.rs`: add `GetCtxRec` enum variant
- `common.rs`: add `GetCtxRecParam`
- `task.rs`: add `get_context_record_content()` session method
- `traits.rs` + `unified.rs`: wire up implementation and tool definition

The tool returns the report file content (if linked) or the brief summary, enabling agents to fetch full details on demand rather than having them embedded in every prompt.

## Key design decisions
- Minimal blast radius: only the Display impls for `for_prompt=true` change; all serialization/parsing logic is unaffected
- No new abstractions: the existing `for_prompt` flag is simply extended to cover rendering, not just data selection
- `get_ctx_rec` follows the identical structure as `delete_ctx_rec` for consistency
