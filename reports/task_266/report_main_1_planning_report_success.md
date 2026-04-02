## Plan: Verify All Prompts at Startup

**Approach:** Add a `validate_all_prompts()` method on `ConfiguredPromptBuilder` that renders every stage's prompt with a dummy task at startup, catching template parse errors and undefined variables early. This replaces the existing `validate_stage_prompts` which only checked file existence.

**Key design decisions:**
- Render with dummy data (same pattern as the existing `prompt` CLI command) to catch template errors without needing real tasks
- Collect all errors before reporting (don't fail on first) for better DX
- Skip `call` stages which don't have their own prompts
- Validate in both startup paths (with/without backends) in commands.rs

**Analog:** The `prompt` CLI command already does single-stage rendering via `build_for_stage_with_task` with `dummy_task_and_comments()`. The new method generalizes this to all stages.

**Checklist items:**
1. Add `validate_all_prompts()` method on `ConfiguredPromptBuilder`
2. Remove dead `validate_stage_prompts` and `file_exists`
3. Call validation at both startup paths in commands.rs
4. Add tests for the new validation