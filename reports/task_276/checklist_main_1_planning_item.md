## What

Update the test `validate_workflow_refs_passes_no_tool_refs` in `zbobr-api/src/config.rs`. After the new validation from step 1, a role with `tool: None` should be rejected by `validate_workflow_refs`.

## Options (pick one)

- **Option A (preferred):** Change the assertion from `is_ok()` to `is_err()` and rename the test to something like `validate_workflow_refs_rejects_role_without_tool`. This serves as the positive test for the new validation.
- **Option B:** Give the role a valid tool (e.g., `tool: Some("smart".to_string())`) and keep the test as a passing-validation test.

Option A is preferred because it doubles as the test for the new validation rule.