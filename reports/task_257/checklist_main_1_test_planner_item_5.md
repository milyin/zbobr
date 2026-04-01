# End-to-end prompt format validation test

**Priority: High**
**File:** `zbobr-api/src/context/mod.rs` (in `mod tests`)

## Rationale

Individual components (MdRecord, MdStage, MdCompactComment) each have unit tests for `for_prompt=true`, but there is no comprehensive test that validates the **complete composed output** of `serialize_context` with `for_prompt=true` against the format specified in the task requirements.

The task requires this specific output shape:
```
- planning
  - 💬 Plan ready for review: bla-bla-bla [ctx_rec_2]
- user milyin: proceed with the plan
- planning
  - ✅ Plan finalized bla bla bla [ctx_rec_9]
    - [x] plan item [ctx_rec_3]
```

## What to test

Create a test `for_prompt_renders_complete_format` that:

1. Builds a `TaskContext` with:
   - Stage "planning" with a 💬 comment record and a checkbox record
   - Stage "working" with NO records (empty — should be filtered out)
   - Stage "reviewing" with a ✅ success record and a [x] checklist item
2. Provides comments interleaved chronologically between stages
3. Calls `serialize_context(&ctx, &comments, true, None)`
4. Validates the **complete output** against expected format:
   - Stage headers are just `- {stage_name}` (no metadata, no timestamp, no model/tool)
   - Records use plain `[ctx_rec_N]` (no `<sub>`, no URLs)
   - Comments are `- user {name}: {body}` (no timestamp, no URL, no bold)
   - Empty "working" stage is filtered out
   - No `<!-- stage -->` markers anywhere
   - Records are properly indented under stages

## Why this matters

This is the only test that validates the complete composition, catching issues like:
- Stage markers leaking into prompt mode (caught by review round 2)
- Non-prompt formatting leaking into prompt mode (caught by review round 1)
- Incorrect interleaving of stages and comments
- Empty stage filtering interacting with comment interleaving
