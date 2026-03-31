In zbobr/src/init.rs, the existing test `default_workflow_includes_test_stages` verifies test stages exist but doesn't check that the preparator stage and role were removed. Add assertions:

- `assert!(!main.stages.contains_key("preparing"))` — preparator stage removed from main pipeline
- `assert!(!workflow.roles.contains_key("preparator"))` — preparator role removed
- Optionally verify `ConfigureWorktree` is not in any role's MCP tool list

This ensures the preparator removal doesn't regress.