## Plan Summary

**Approach:** Add a validation check in `validate_workflow_refs()` that rejects any role where `tool` is `None`, catching misconfigurations at startup instead of runtime. Then fix all 13 compilation errors (11 in zbobr-api tests, 2 in zbobr-dispatcher integration tests) caused by references to the removed global `tool` field on `ZbobrDispatcherConfig`.

**Key Design Decisions:**
- Keep `RoleDefinition.tool` as `Option<String>` for serde flexibility — enforcement is purely at the validation layer
- Reuse the existing error pattern in `validate_workflow_refs` for the new check
- Tests that validated the removed global tool fallback are either removed (if testing removed behavior) or converted to error-case tests

**Analog:** The existing checks in `validate_workflow_refs()` itself — same function, same error style, same bail pattern.

**Checklist items created:**
1. Add validation requiring every role to have `tool` defined
2. Fix 4 `resolve_tool_name` tests
3. Remove 3 obsolete global-tool validation tests
4. Update `validate_workflow_refs_passes_no_tool_refs` test
5. Fix 2 zbobr-dispatcher integration tests
6. Verify all tests compile and pass