# Implementation Report: Verify Prompts on Start

## Changes Made

### 1. Added `validate_all_prompts()` method (`zbobr-dispatcher/src/prompts.rs`)
- New method on `ConfiguredPromptBuilder` that creates a dummy `Task` and empty comments
- Iterates all stages via `self.workflow.config().all_stages()`
- Skips `call` stages (no prompts to validate)
- Calls `build_for_stage_with_task()` for each remaining stage
- Collects all errors with pipeline/stage context and reports them together
- Returns `Ok(())` if all prompts render successfully

### 2. Removed dead code (`zbobr-dispatcher/src/prompts.rs`, `zbobr-dispatcher/src/lib.rs`)
- Removed `validate_stage_prompts()` function (only checked file existence, never rendered templates)
- Removed `file_exists()` helper function
- Removed `validate_stage_prompts` from the `pub use` export in `lib.rs`

### 3. Called validation at startup (`zbobr/src/commands.rs`)
- Added `prompt_builder.validate_all_prompts()?` in the without-backends path (line ~209, before `run_without_backends`)
- Added `prompt_builder.validate_all_prompts()?` in the with-backends path (line ~221, before building the dispatcher)

### 4. Tests
- Skipped per workflow rules: "do NOT implement new tests for new functionality in this stage"
- All existing tests pass (`cargo test --workspace`)

## Commits
1. `624296ba` - add validate_all_prompts() method on ConfiguredPromptBuilder
2. `8537c08e` - remove dead validate_stage_prompts and file_exists
3. `7422a1a8` - call validate_all_prompts() at startup in both code paths
