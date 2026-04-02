## What to change

In `zbobr-api/src/config.rs`:
- In `ZbobrDispatcherConfig::validate()`, add a check that `self.tool` (the global default tool name) exists as a key in `self.tools`. Fail with a clear error if it doesn't.
- Add a new method `ZbobrDispatcherConfig::validate_workflow_refs(&self, workflow: &WorkflowConfig) -> anyhow::Result<()>` that:
  - Iterates all `RoleDefinition` entries in the workflow and rejects any `Some(tool)` in `role.tool` that is not a key in `self.tools`.
  - Iterates all pipeline stage definitions and rejects any `Some(tool)` in `stage.tool` that is not a key in `self.tools`.

In `zbobr-dispatcher/src/lib.rs`:
- In `ZbobrDispatcher::validated()`, after `self.config.validate()?`, call `self.config.validate_workflow_refs(self.workflow.config())?`.
- `Workflow::config()` getter already exists at `zbobr-dispatcher/src/workflow.rs` — use it directly.

## Why
The providers/tools refactor replaced direct `tool/model/plan_mode` fields with named tool references. Without eager validation, a typo in `dispatcher.tool`, a role's `tool`, or a stage's `tool` passes startup and only fails mid-execution. Checking all references at config load time catches misconfiguration early, consistent with the rest of validate()'s role.