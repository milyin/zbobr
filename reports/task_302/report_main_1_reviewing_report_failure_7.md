Review result: **changes are mostly consistent with the chosen analogs, but the branch introduces one blocking regression that should be fixed before merge.**

## Overall assessment
- The main multi-config implementation follows the planned analogs well:
  - `ConfigFileArg` -> repeatable `Vec<PathBuf>` with `-c`/`--config`
  - `merge_toml` generation in the macro mirrors the existing `merge_with_args` pattern
  - map sections are merged key-wise with recursive merge for matching keys
  - config loading in `zbobr/src/main.rs` folds configs in appearance order and resolves paths per config file before merging
- The added tests are generally valuable and aligned with the feature: they lock down `Option<Vec<_>>` semantics, key-wise map merging for tools, and end-to-end TOML merge behavior.

## Blocking issue
### 1. Unrelated runtime behavior change in prompt MCP variable injection
`zbobr-dispatcher/src/prompts.rs:314-318` and `348-352` changed the default from:
- previous behavior: if a role had no explicit `mcp` list, prompt rendering exposed **all** static MCP variables
- new behavior: if `mcp` is absent, prompt rendering exposes **no** MCP variables (`unwrap_or(&[])`)

Why this is a problem:
- This is a behavioral change outside the task scope. The task is about loading and merging multiple config files, not changing the runtime meaning of an omitted `mcp` field.
- It is a compatibility regression for existing configs that relied on the old default. Prompts using placeholders like `{mcp_report_success}` will now fail for roles whose `mcp` was simply omitted.
- The branch also removed `McpTool::all()` from `zbobr-api/src/config_tools.rs:17-31` and updated tests/helpers to explicitly populate `Some(ALL_TOOLS.to_vec())` (`zbobr-dispatcher/tests/mcp_integration/abstract_test_helpers.rs:84-89`), which masks the regression rather than preserving previous behavior.

Suggested fix:
- Restore the previous fallback in prompt rendering when `role_definition(role_name)` is missing or `role.mcp` is `None`.
- If `McpTool::all()` was removed only for this refactor, either restore it or use `ALL_TOOLS` directly to keep the default behavior explicit and centralized.
- Keep the new merge semantics (`None` = inherit, `Some(vec![])` = explicit clear) without changing the runtime fallback for configs that never set `mcp`.

## Analog consistency
- **Good:** `merge_toml` in the proc macro is a strong analog to `merge_with_args`, and the manual `WorkflowToml::merge_toml` follows the same recursive/key-wise pattern as `PipelineConfig`.
- **Good:** resolving each parsed config before folding is consistent with the intended shared-base/project-overlay design.
- **Bad / out of scope:** the prompt-layer `mcp` fallback change is not part of that analog and introduces behavior drift unrelated to multi-config support.

## Checklist status
All checklist items in the provided context were already marked complete, and the implemented work for those items appears present in the branch. The failure is due to the unrelated regression above, not missing checklist implementation.