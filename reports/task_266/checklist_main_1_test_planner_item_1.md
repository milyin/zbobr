## Test: `validate_all_prompts_multi_pipeline`

**File:** `zbobr-dispatcher/src/prompts.rs` (test module)

**Purpose:** Verify that `validate_all_prompts()` validates stages across ALL pipelines (not just "main"), and that error messages correctly identify the pipeline name. The current `make_prompt_builder()` helper only creates a single "main" pipeline — this test needs a modified setup with multiple pipelines.

**Setup:**
- Modify or create a variant of `make_prompt_builder()` that accepts a `HashMap<Pipeline, PipelineConfig>` (or build the config manually)
- Create two pipelines: "main" with a valid stage, and "secondary" with a stage that has a missing prompt file
- Call `validate_all_prompts()`

**Assertions:**
- `validate_all_prompts()` returns `Err`
- The error message contains `"pipeline 'secondary'"` — confirming the function iterated beyond "main" and correctly identifies the failing pipeline
- Optionally also assert the stage name appears

**Note:** The `make_prompt_builder()` helper may need to be generalized to accept a full `HashMap<Pipeline, PipelineConfig>` instead of just an `IndexMap<Stage, StageDefinition>`. Keep the existing helper and add an overload or build the config inline in this test.