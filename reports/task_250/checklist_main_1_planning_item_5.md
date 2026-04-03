In `zbobr/src/init.rs`, update existing tests to account for the new linting stage and linter role.

**What to update:**

1. **`default_prompts_reference_get_ctx_rec_when_role_has_access` test** (around line 930): Add `LINTER_PROMPT` to the list of prompts being checked, since the linter role has GetCtxRec access and the prompt must reference it.

2. **Verify existing tests still pass:** The `default_workflow_roles_have_tool` test should automatically pass since the new "linter" role has a tool defined. The `default_workflow_includes_test_stages` test should also still pass as it checks specific stages that haven't changed.

**Why:** The linter prompt uses get_ctx_rec_guidance!() macro and the role has GetCtxRec in its MCP tools, so it must be included in the prompt validation test.