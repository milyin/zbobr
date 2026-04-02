Add a `pub fn validate_all_prompts(&self) -> anyhow::Result<()>` method on `ConfiguredPromptBuilder` in `zbobr-dispatcher/src/prompts.rs`.

**What it does:**
- Creates a dummy `Task` (populate all standard fields with placeholder strings) and an empty `Vec<Comment>` (or with a dummy comment)
- Iterates `self.workflow.config().all_stages()` which returns `Vec<(&Pipeline, &str, &StageDefinition)>`
- Skips `call` stages (where `stage_def.is_call()` returns true) — these don't have prompts
- Calls `self.build_for_stage_with_task(stage_def, &task, &comments)` for each remaining stage
- Collects all errors (don't fail on first — collect pipeline/stage name with each error for context)
- If any errors were collected, return a combined error listing all failures; otherwise Ok(())

**Why:** The existing `validate_stage_prompts` only checks file existence but never renders templates. This new method actually renders every prompt with a dummy task, catching template parse errors and undefined variables at startup rather than at runtime.

**Analog:** Follow the pattern already used by the `prompt` CLI command in `commands.rs` which uses `dummy_task_and_comments()` and calls `build_for_stage_with_task`. The dummy task helper in `prompts.rs` tests (`dummy_task()`) is also a good reference. Reuse the existing `dummy_task_and_comments()` from commands.rs or create a similar one local to prompts.rs.