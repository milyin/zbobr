Overall assessment: the implementation is correct and complete for this task.

Analog consistency:
- The planner chose an appropriate analog: extending `ZbobrDispatcherConfig::validate_workflow_refs()` alongside the existing role/stage tool-reference checks.
- The implementation follows that pattern cleanly by adding the new role-level presence check before the existing existence check.
- The follow-up fix in `zbobr/src/init.rs` is also consistent with the new invariant: the generated default workflow now supplies explicit tools for all predefined roles.

What I reviewed:
1. `zbobr-api/src/config.rs`
   - `validate_workflow_refs()` now rejects roles without `tool` defined via `Role '<name>' has no tool defined`.
   - Existing unknown-tool validation remains intact.
   - `resolve_tool_name` documentation was updated to match actual behavior after removal of the global dispatcher-level fallback.
   - Test updates are consistent with the removed global fallback and the new validation rule.
2. `zbobr/src/init.rs`
   - All predefined roles that previously relied on `..Default::default()` for `tool` now have explicit tool assignments.
   - The assigned tool names match tool definitions already present in `default_config_toml()` (`smart`, `planner`, `silly`), so the generated config remains internally consistent.
3. `zbobr-dispatcher/tests/mcp_integration/env.rs`
   - Removal of the obsolete dispatcher-level `tool` field references is directly related to the task and not extraneous.

Findings:
- No blocking issues found.
- No unrelated or unnecessary changes found in the branch diff.
- The previous review regression (default workflow becoming invalid under the new validation rule) is resolved.

Checklist status:
- All checklist items in the task context are already checked and the current implementation matches that completion state.

Conclusion:
- The task requirements are satisfied.
- The implementation is consistent with the selected analog and current codebase behavior.
- Review passed.