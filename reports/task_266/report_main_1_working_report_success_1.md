## What was done

Added the 4 unit tests for `validate_all_prompts()` in `zbobr-dispatcher/src/prompts.rs` that the review identified as missing.

### Tests added

1. **`validate_all_prompts_valid_templates_pass`** — Creates a workflow with a valid prompt file using `{title}`, verifies validation returns Ok.
2. **`validate_all_prompts_undefined_variable_fails`** — Creates a prompt with `{mcp_nonexistent}`, verifies validation returns an error mentioning the bad variable.
3. **`validate_all_prompts_missing_file_fails`** — References a non-existent prompt file, verifies validation returns an error about the missing file.
4. **`validate_all_prompts_call_stages_skipped`** — Creates a workflow with a `call` stage (no prompt files), verifies validation doesn't error on it.

### Implementation details

- Added a `make_prompt_builder()` test helper that constructs a `ConfiguredPromptBuilder` from an `IndexMap` of stages, using `Workflow::from_config()` to skip workflow validation (appropriate for unit tests).
- Tests follow existing patterns: `TempDir` for prompt files, `write_file` helper, direct `WorkflowConfig` construction.
- All 21 tests in the module pass.

### Commit

`367a23b6` — add unit tests for validate_all_prompts()