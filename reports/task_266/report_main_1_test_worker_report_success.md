## Tests Implemented

### 1. `validate_all_prompts_aggregates_multiple_errors` (ctx_rec_12)
- **File:** `zbobr-dispatcher/src/prompts.rs`
- **Purpose:** Confirms that `validate_all_prompts()` collects errors from ALL failing stages rather than stopping at the first failure.
- **Setup:** Two stages — `stage_a` with a missing prompt file, `stage_b` with an undefined template variable.
- **Assertions:** The error string contains both `stage_a` and `stage_b`, proving aggregation works.

### 2. `validate_all_prompts_multi_pipeline` (ctx_rec_13)
- **File:** `zbobr-dispatcher/src/prompts.rs`
- **Purpose:** Confirms validation iterates across all pipelines (not just "main") and error messages identify the pipeline name.
- **Setup:** Two pipelines — "main" with a valid stage, "secondary" with a stage referencing a missing prompt file. Added `make_prompt_builder_multi()` helper accepting a full `HashMap<Pipeline, PipelineConfig>`.
- **Assertions:** Error contains `"pipeline 'secondary'"` and `"stage 'broken'"`.

### Test Results
All 6 `validate_all_prompts` tests pass (total 73 tests in the crate).

### Commit
`e99d9c53` — add tests for error aggregation and multi-pipeline validation