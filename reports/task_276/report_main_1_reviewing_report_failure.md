Overall assessment: the analog choice was correct — adding the new guard directly inside `ZbobrDispatcherConfig::validate_workflow_refs()` follows the existing validation pattern and the updated tests in `zbobr-api/src/config.rs` are consistent with the removed global fallback. However, the change is incomplete and introduces a functional regression in the default workspace configuration.

Findings:

1. **Blocking regression: generated default workflow is now invalid**
   - `zbobr-api/src/config.rs:663-667` now rejects every role with `tool = None`.
   - But `zbobr/src/init.rs:333-417` still creates predefined roles `worker`, `test_planner`, `test_worker`, `reviewer`, `tester`, and `merger` via `..Default::default()`, leaving `tool` unset.
   - That default workflow is serialized by `default_config_toml()` (`zbobr/src/init.rs:91-92`, `216-220`) and then used by normal command startup, which validates the workflow before running (`zbobr/src/commands.rs:200` and `225-235`).
   - Result: a freshly initialized workspace can now fail validation before use, because the generated config violates the new invariant.
   - Suggested fix: assign an explicit tool to every predefined role in `default_workflow()` and add/adjust an init/default-workflow test so this invariant is covered.

2. **Documentation drift in the changed code path**
   - `zbobr-api/src/config.rs:694` still documents `resolve_tool_name` as `stage.tool → role.tool → global dispatcher.tool`, but the global fallback has been removed and the function now errors instead (`709-712`).
   - This is not the main regression, but it is directly related to the task and should be updated so the code comments match the actual behavior.

Analog consistency:
- The planner’s analog was appropriate: the implementation reused the existing `validate_workflow_refs()` validation style and error shape.
- The inconsistency is not in the core validation change itself, but in failing to propagate the new invariant to the default workflow generator and the nearby behavior documentation.

Checklist status:
- The task context marks all checklist items complete, but based on the regression above the implementation is not ready to accept as correct.