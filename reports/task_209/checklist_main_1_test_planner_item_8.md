# Tests for validate_workflow_refs() — zbobr-api/src/config.rs

The latest commit added `validate_workflow_refs(&self, workflow: &WorkflowConfig) -> anyhow::Result<()>` which checks that every `role.tool` and `stage.tool` reference exists in `self.tools`. There are currently **zero tests** for this method.

## Tests to add (in existing `#[cfg(test)] mod tests` in config.rs)

### 1. `validate_workflow_refs_rejects_unknown_role_tool`
- Setup: config with `tools = {"smart": [...]}`, workflow with role `worker` having `tool = Some("nonexistent")`
- Assert: `validate_workflow_refs()` returns error containing "Role 'worker' references unknown tool"

### 2. `validate_workflow_refs_rejects_unknown_stage_tool`
- Setup: config with `tools = {"smart": [...]}`, workflow with pipeline `main` having stage `working` with `tool = Some("bad")`
- Assert: `validate_workflow_refs()` returns error containing "Stage 'main/working' references unknown tool"

### 3. `validate_workflow_refs_passes_valid_refs`
- Setup: config with `tools = {"smart": [...]}`, workflow with role `worker` having `tool = Some("smart")` and a stage with `tool = Some("smart")`
- Assert: `validate_workflow_refs()` succeeds

### 4. `validate_workflow_refs_passes_no_tool_refs`
- Setup: config with `tools = {"smart": [...]}`, workflow with role and stage both having `tool = None`
- Assert: `validate_workflow_refs()` succeeds (None refs don't need validation)

## Implementation notes
- Use existing `make_config()` and `make_workflow_with_role()` helpers for roles.
- For stage tests, construct a `WorkflowConfig` with a pipeline containing a `StageDefinition` directly.
- These are all pure unit tests.
