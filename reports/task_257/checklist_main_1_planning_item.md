## What to change

Modify `zbobr-api/src/context/mod.rs` to simplify what agents see when `for_prompt=true`.

### Target format
```
- planning
  - 💬 Plan ready for review: text [ctx_rec_2]
- user milyin: proceed with the plan
- planning
  - ✅ Plan finalized [ctx_rec_9]
    - [x] plan item [ctx_rec_3]
```

### Changes needed

**`MdRecord`** — add `for_prompt: bool` field:
- In `Display`: when `for_prompt=true`, render `[ctx_rec_N]` as plain text instead of `<sub>ctx_rec_N</sub>` (no URL link, no HTML sub tag).
- Update `from_context_record(r, report_url)` to accept `for_prompt: bool` and set it on the struct.

**`MdStage`** — add `for_prompt: bool` field:
- In `Display`: when `for_prompt=true`, render the stage header as `- {stage_name}` (just the stage name, no instance/pipeline/run_id/tool/model/timestamp). Also pass `for_prompt` to each `MdRecord` when rendering.
- In `from_stage_context(stage, for_prompt, report_url)`: set `for_prompt` on the struct and pass it into `from_context_record`.

**`MdCompactComment`** — add `for_prompt: bool` field:
- In `Display`: when `for_prompt=true`, render as `- user {username}: {text}` without timestamp and without link.
- Update `from_comment(c, for_prompt)` to set this field.

**`MdContext`** — add `for_prompt: bool` field:
- In `Display`: when `for_prompt=true`, skip `MdEntry::Stage` entries that have zero records.
- Update `from_task_context` to set `for_prompt` on the context, and propagate it to all stages and comments.

### Why
Stage metadata (instance, pipeline, run_id, tool, model, timestamp) is routing/audit info that agents don't need in their prompt. The `<sub>ctx_rec_N</sub>` HTML syntax is noisy; plain `[ctx_rec_N]` is cleaner and agents can call `get_ctx_rec` to retrieve linked content. Empty stages (typically failed stages with no records) add noise without value.

### Pattern to follow
The `for_prompt` flag already flows through `from_stage_context` — extend that flow to affect Display rendering rather than just data filtering.
