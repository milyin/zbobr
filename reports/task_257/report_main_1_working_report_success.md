# Implementation Complete

## Changes Made

### 1. Simplified for-prompt context rendering (`zbobr-api/src/context/mod.rs`)

Added `for_prompt: bool` field to `MdRecord`, `MdCompactComment`, `MdStage`.

**Display changes when `for_prompt=true`:**
- `MdStage`: renders as `- {stage_name}` only (no instance/pipeline/run_id/tool/model/timestamp)
- `MdRecord`: renders as `[ctx_rec_N]` (plain text, no `<sub>` HTML, no URL)
- `MdCompactComment`: renders as `- user {username}: {text}` (no timestamp, no link)
- Empty stages (no records) are filtered out by `MdContext::from_task_context`

### 2. `get_ctx_rec` MCP tool (4-file change)

- **`zbobr-api/src/config_tools.rs`**: Added `GetCtxRec` variant to `McpTool` enum, `as_str()`, `FromStr`, `ALL_TOOLS`, `ALL_TOOL_NAMES`
- **`zbobr-dispatcher/src/mcp/common.rs`**: Added `GetCtxRecParam` struct
- **`zbobr-dispatcher/src/task.rs`**: Added `get_context_record_content(record_id)` method on `RoleSession` — returns report file content if present, otherwise the brief
- **`zbobr-dispatcher/src/mcp/traits.rs`**: Added `get_ctx_rec_impl` following `delete_ctx_rec_impl` pattern
- **`zbobr-dispatcher/src/mcp/unified.rs`**: Wired up `get_ctx_rec` tool with description

### 3. Role configuration (`zbobr/src/init.rs`)

Added `GetCtxRec` to allowed tools for: planner, worker, test_planner, test_worker, reviewer, tester.

## Tests Updated

- `serialize_for_prompt_omits_prompt_link` — updated to check new format
- `roundtrip_for_prompt_loses_prompt_link` — updated (for_prompt output not parseable back)
- `for_prompt_also_omits_output_link` — updated similarly
- `for_prompt_true_uses_compact_comment_format` — updated (no timestamp in prompt mode)
- `for_prompt_true_does_not_truncate_long_comment_text` — updated
- All struct literal initializers updated with `for_prompt: false`

One pre-existing test failure (`compact_comment_uses_first_line_only`) was present before these changes and is unrelated.
