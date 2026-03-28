**Review complete.** The implementation successfully renames the ERROR section to STATUS and unifies the pause-with-status mechanism. All 6 checklist items are verified correct:

1. ✅ Error field renamed to status throughout the codebase
2. ✅ ERROR_SEPARATOR renamed to STATUS_SEPARATOR in backends
3. ✅ Shared status-formatting function with icon+timestamp enforcement
4. ✅ RoleSession/TaskSession updated with new pause API
5. ✅ stop_with_error/question unified via shared pause_with_status_impl
6. ✅ All cli.rs pause callers updated to use new API

The implementation correctly enforces that **pause cannot be set without a status explanation** at the API level. Questions are properly placed in both the STATUS section and agent report (via context records), while errors appear only in the STATUS section. No issues found.