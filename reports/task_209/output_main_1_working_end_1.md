Both review issues are now fixed:

1. **Priority inheritance** — `ProviderDefinition.priority` is now `Option<i32>`. A child without an explicit priority inherits from its parent; root providers default to 10. Previously, omitting `priority` was silently the same as `priority = 10`, breaking inherited fallback ordering.

2. **Executor validation** — `validate()` now rejects unknown executor strings (must be `claude`, `copilot`, or `mcp-tester`). `build_executor()` returns `Result` and errors on unknown executors instead of defaulting to Claude. A typo like `executor = "claud"` now fails fast at validation time.

All 20 existing tests pass. Changes committed as `ae85fd88`.