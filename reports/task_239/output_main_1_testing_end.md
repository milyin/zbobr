Testing complete! ✅

## Summary

I've conducted comprehensive testing of the instance filtering implementation. Here are the results:

**Test Execution:**
- **Unit Tests:** 94 passed (zbobr-api, zbobr-dispatcher, zbobr-task-backend-github, zbobr-executor-mcp-tester)
- **Integration Tests:** 15 passed (fs_fs backend tests)
- **GitHub Integration Tests:** 9 skipped (requires external credentials)
- **Build:** Successful with no errors
- **Linting:** No new clippy warnings introduced

**Implementation Verification:**
All 6 checklist items have been correctly implemented:
1. ✅ `instance: String` field added to `ZbobrDispatcherConfig`
2. ✅ `instance` injected into GitHub backend config at runtime
3. ✅ `zbobr:<instance>` label created in setup; force cleanup of other instance labels
4. ✅ Tasks filtered by instance label in `list_tasks`
5. ✅ Stage title format updated to `instance:pipeline:run_id:**stage**`
6. ✅ Instance populated from dispatcher config in CLI

**Result:** All 109 tests pass, zero failures. The implementation is complete and ready for merge.