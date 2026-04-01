# Plan: Remove Noise from Context for Prompt

## Context

When context is rendered for agent prompts, it currently includes verbose stage metadata (instance, pipeline, run ID, tool, model, timestamp, links) and uses HTML `<sub>` tags for ctx_rec IDs with embedded URLs. This adds noise that makes context harder to read. The task requires:
1. Simplifying the prompt rendering of stages, records, and comments
2. Adding a `get_ctx_rec` MCP tool so agents can fetch report content on demand

The `for_prompt: bool` flag already flows through the context serialization chain (`serialize_context` → `MdContext::from_task_context` → `MdStage::from_stage_context` → `MdCompactComment::from_comment`). We extend it further into the individual element types.

## Desired Prompt Format

```
- planning
  - 💬 Plan ready for review: bla-bla-bla [ctx_rec_2]
- user milyin: proceed with the plan
- planning
  - ✅ Plan finalized bla bla bla [ctx_rec_9]
    - [x] plan item [ctx_rec_3]
```

vs. current storage format:
```
- skynet:main:1:**planning** `claude` `claude-sonnet-4.6` `2026-04-01 16:46:50 +0200`
  - 💬 Plan ready for review: bla-bla-bla <sub>[ctx_rec_2](reports/plan.md)</sub>
- user:**milyin** proceed with the plan `2026-04-01 16:46:50 +0200`
```

## Changes

### 1. Context serialization — `zbobr-api/src/context/mod.rs`

Three struct types need a `for_prompt: bool` field:

**`MdRecord`**:
- Add `for_prompt: bool`
- Update `from_context_record(r, report_url)` → `from_context_record(r, report_url, for_prompt)` to store the flag
- Update `Display`: when `for_prompt`, emit `{prefix}{brief} [ctx_rec_N]` instead of `{prefix}{brief} <sub>ctx_rec_N</sub>` / `<sub>[ctx_rec_N](url)</sub>`
- `FromStr` stays unchanged (only parses stored format)

**`MdStage`**:
- Add `for_prompt: bool`
- `from_stage_context` already receives `for_prompt`; store it and pass it to `MdRecord::from_context_record`
- Update `Display`: when `for_prompt`, write `- {stage_name}\n` instead of `- {full MdStageTitle}\n`

**`MdCompactComment`**:
- Add `for_prompt: bool`
- `from_comment` already receives `for_prompt`; store it
- Update `Display`: when `for_prompt`, write `- user {username}: {text}` (no timestamp, no link, no bold); keep existing format when not for_prompt

Update unit tests in `mod.rs` that check `for_prompt = true` output to match new format.

### 2. New MCP tool `get_ctx_rec`

**`zbobr-api/src/config_tools.rs`**:
- Add `GetCtxRec` to `McpTool` enum, `ALL_TOOLS`, `ALL_TOOL_NAMES`
- Follow the `DeleteCtxRec` → `delete_ctx_rec` naming convention

**`zbobr-dispatcher/src/mcp/common.rs`**:
- Add `GetCtxRecParam` struct with `id: String` field (same as `DeleteCtxRecParam`)

**`zbobr-dispatcher/src/mcp/traits.rs`**:
- Add `get_ctx_rec_impl(&self, id_str: &str) -> String`
- Parse ID via `parse_ctx_rec_id`
- Read full task via `self.session().get_task().await` (already public on `RoleSession`)
- Find record via `task.context.find_record(record_id)`
- If record not found: return error
- If `report_link` is None: return brief text with a note that no report is attached
- If `report_link` is Some(filename): call `self.session().read_report(&filename).await` and return the file content

**`zbobr-dispatcher/src/mcp/unified.rs`**:
- Import `GetCtxRecParam`
- Add `get_ctx_rec` tool with `#[tool(description = "...")]` following the pattern of `delete_ctx_rec`

**`zbobr/src/init.rs`**:
- Add `GetCtxRec` to all roles' `mcp` tool lists (planner, worker, test_planner, test_worker, reviewer, tester, merger — all roles that will receive context with `[ctx_rec_N]` IDs)

## Verification

1. Run `cargo test -p zbobr-api` — context serialization tests must pass with updated format assertions
2. Run `cargo test -p zbobr-dispatcher` — MCP tool tests including the new `get_ctx_rec` must pass
3. Run `cargo test` — full test suite green
4. Manually inspect a rendered prompt context to confirm simplified format
