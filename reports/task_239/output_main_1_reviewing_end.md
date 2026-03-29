## Review Complete

The implementation is **ready** and meets all requirements. All 6 checklist items are correctly implemented:

1. ✅ **Instance config field** - Added to `ZbobrDispatcherConfig` with default "default"
2. ✅ **Backend injection** - Instance injected from dispatcher to GitHub backend config at runtime
3. ✅ **Label creation** - Creates `zbobr:<instance>` label with force cleanup for other instances
4. ✅ **Task filtering** - Filters issues by instance label in list_tasks
5. ✅ **Stage title format** - Updated to `instance:pipeline:run_id:**stage**` with proper parsing
6. ✅ **Instance population** - Correctly cloned from dispatcher config when creating StageInfo

The code compiles successfully and all tests include proper instance field initialization. The implementation correctly allows multiple zbobr instances to run in parallel on the same repository, each managing its own task pool via GitHub labels.