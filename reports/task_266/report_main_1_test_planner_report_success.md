## Test Plan Analysis

### Existing Coverage (4 tests added in implementation)
The branch already includes 4 unit tests for `validate_all_prompts()`:
1. `validate_all_prompts_valid_templates_pass` — valid templates render OK
2. `validate_all_prompts_undefined_variable_fails` — undefined variable caught
3. `validate_all_prompts_missing_file_fails` — missing prompt file caught
4. `validate_all_prompts_call_stages_skipped` — call stages skipped without error

These cover the basic positive and negative paths well.

### Gaps Identified
Two behaviors **unique to `validate_all_prompts()`** lack test coverage:

1. **Error aggregation** (lines 93-105 of prompts.rs): The function collects errors from ALL failing stages and reports them together. No existing test verifies that multiple errors are aggregated rather than short-circuiting on the first failure. This is a key design choice of the function.

2. **Multi-pipeline iteration**: The function iterates `self.workflow.config().all_stages()` which crosses all pipelines. All 4 existing tests use a single "main" pipeline via the `make_prompt_builder()` helper. No test verifies that stages in non-main pipelines are validated, or that error messages correctly identify the pipeline name in the format `"pipeline 'X', stage 'Y': ..."`.

### Checklist Items Added
- Add test: multiple errors are aggregated across stages (not first-failure-stops)
- Add test: validation iterates across multiple pipelines and error identifies pipeline name

### What Was NOT Flagged
- `with_var()` interaction, `prompts_dir` resolution, role-based tool filtering — these are behaviors of the underlying `build_for_stage_with_task()` / `build_prompt_with_task()` functions, not unique to `validate_all_prompts()`. They are already covered (or should be covered) by tests for those lower-level functions.
- Integration tests for startup validation call — the two lines in `commands.rs` are trivial wiring (`prompt_builder.validate_all_prompts()?`), and integration tests for this would require significant infrastructure for minimal additional confidence.