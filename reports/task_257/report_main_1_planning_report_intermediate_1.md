# Plan: Remove noise from context for prompt

## Summary

Simplify how task context is rendered when `for_prompt=true` so agents receive clean, minimal context. Also add a `get_ctx_rec` MCP tool for agents to fetch linked report content on demand.

## Desired format (for_prompt=true)

```
- planning
  - 💬 Plan ready for review: bla-bla-bla [ctx_rec_2]
- user milyin: proceed with the plan
- planning
  - ✅ Plan finalized bla bla bla [ctx_rec_9]
    - [x] plan item [ctx_rec_3]
```

vs current noisy format:
```
- skynet:main:1:**planning** `claude` `claude-sonnet-4.6` `2026-04-01 16:46:50 +0200`
  - 💬 Plan ready for review: bla-bla-bla <sub>[ctx_rec_1](url)</sub>
- user:**milyin** message text `2026-04-01 14:48:12 +0000` <sub>[link](url)</sub>
```

## Changes

### 1. Simplify stage title for prompts
**`zbobr-api/src/context/stage_title.rs`**
Add a for-prompt display wrapper rendering only the stage name (no instance/pipeline/run_id/tool/model/timestamp).

### 2. Add `for_prompt` to private context structs + change Display
**`zbobr-api/src/context/mod.rs`**
- Add `for_prompt: bool` to `MdStage`, `MdRecord`, `MdCompactComment`
- `MdStage::fmt` when `for_prompt`: output `- stage_name\n` only
- `MdRecord::fmt` when `for_prompt`: output `[ctx_rec_N]` plain text instead of `<sub>...</sub>` HTML
- `MdCompactComment::fmt` when `for_prompt`: output `- user username: text` (no timestamp, no link)
- `MdContext::from_task_context` when `for_prompt`: filter out stages with no records

### 3. Add `GetCtxRec` MCP tool
**`zbobr-api/src/config_tools.rs`** — add `GetCtxRec` variant  
**`zbobr-dispatcher/src/task.rs`** — add `get_context_record_content(record_id)` on `RoleSession`  
**`zbobr-dispatcher/src/mcp/traits.rs`** — add `get_ctx_rec_impl` (mirrors `delete_ctx_rec_impl`)  
**`zbobr-dispatcher/src/mcp/unified.rs`** — register `get_ctx_rec` tool (reuses `DeleteCtxRecParam`)

## Verification
- `cargo test` in `zbobr-api` (for-prompt context tests will need updating)
- Manual check that for-prompt output matches desired format
- Verify `get_ctx_rec` returns file content for records with a report_link
