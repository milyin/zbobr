# Plan: Verify All Prompts at Startup

## Context

Currently `validate_stage_prompts()` only checks that prompt files **exist**, but never actually parses or renders them. Template parse errors and undefined variable errors (e.g., `{mcp_nonexistent}`) are only caught at **runtime** when a stage executes — potentially hours after startup. The goal is to catch all prompt incorrectness at startup by rendering every stage's prompt with a dummy task.

Key observation: the `prompt` CLI command already does exactly this for a single stage using `build_for_stage_with_task(stage_def, &task, &comments)`. We need to do the same for ALL stages at startup.

## Approach

### 1. Add `validate_all_prompts()` method on `ConfiguredPromptBuilder`

**File:** `zbobr-dispatcher/src/prompts.rs`

Add a `pub fn validate_all_prompts(&self) -> anyhow::Result<()>` method that:
- Creates a dummy `Task` (with all standard fields populated) and dummy `Comments`
- Iterates `self.workflow.config().all_stages()`
- Skips `call` stages (`stage_def.is_call()`)
- Calls `self.build_for_stage_with_task(stage_def, &task, &comments)` for each
- Collects all errors with pipeline/stage context, reports them together

The dummy task should populate: `title`, `description`, `work_branch`, `context` (via `TaskContext::default()`). Extra vars like `destination_repository`/`destination_branch` are already on the builder.

### 2. Remove dead `validate_stage_prompts` function

**Files:** `zbobr-dispatcher/src/prompts.rs`, `zbobr-dispatcher/src/lib.rs`

Remove `validate_stage_prompts()` and the helper `file_exists()` — they have zero callers and are superseded by the new validation. Remove the export from `lib.rs`.

### 3. Call `validate_all_prompts()` at startup

**File:** `zbobr/src/commands.rs`

Call `prompt_builder.validate_all_prompts()?` after `ConfiguredPromptBuilder` construction in both code paths:
- **Without backends** (line ~208): before `return run_without_backends(...)`
- **With backends** (line ~219): before building the dispatcher

### 4. Add tests

**File:** `zbobr-dispatcher/src/prompts.rs` (test module)

- Valid templates render OK
- Undefined variable caught (e.g., `{mcp_nonexistent}`)
- Missing prompt file caught
- `call` stages skipped without error

## Verification

1. `cargo build` — compiles
2. `cargo test -p zbobr-dispatcher` — new and existing tests pass
3. `cargo test --workspace` — no regressions
4. Manual: introduce a bad placeholder in a prompt file, run any command, verify the error appears immediately at startup
