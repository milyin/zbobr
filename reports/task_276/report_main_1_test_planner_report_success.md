## Test Planning Report

### Summary
One new test is needed. All other implementation changes have adequate existing coverage.

### Analysis

**Well-covered (no new tests needed):**
- `validate_workflow_refs` rejection of tool-less roles → `validate_workflow_refs_rejects_role_without_tool`
- `validate_workflow_refs` acceptance of valid config → `validate_workflow_refs_passes_valid_refs`
- `resolve_tool_name` error paths → `resolve_tool_name_errors_when_no_tool`, `resolve_tool_name_errors_when_no_role`
- `resolve_tool_name` precedence → `resolve_tool_name_stage_overrides`, `resolve_tool_name_falls_back_to_role`
- Integration test env construction → compiles correctly without removed global `tool` field

**Gap found:**
- `default_workflow()` roles having `tool` defined — this was the exact regression caught during code review (ctx_rec_10). No test prevents it from recurring.

### New Test

**`default_workflow_roles_have_tool`** in `zbobr/src/init.rs::tests`
- Iterates all roles from `default_workflow()` and asserts each has `tool: Some(...)`
- Directly guards against the review-caught regression
- Verification: `cargo test -p zbobr -- default_workflow_roles_have_tool`