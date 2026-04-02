Done. The two review findings are fixed:

1. **`zbobr/src/init.rs`**: All 6 predefined roles in `default_workflow()` now have an explicit `tool` — `worker`/`test_worker`/`reviewer`/`tester` use `"smart"`, `test_planner` uses `"planner"`, `merger` uses `"silly"`.
2. **`zbobr-api/src/config.rs`**: `resolve_tool_name` doc comment updated to reflect the removed global fallback.