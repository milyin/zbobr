## Update example config in `zbobr/src/init.rs`

### 2a. Simplify the example providers and tools

In `default_config_toml()`:
- Remove `copilot_planner` from the providers map (keep `claude`, `copilot`, `claude_planner`)
- Rename the tools to match task description: use `developer` (instead of `smart`) and adjust entries
- Add a `ToolEntry` with `priority: Some(0)` to demonstrate the new field (e.g., copilot entry as fallback)
- Update the `planner` tool entries to remove references to `copilot_planner` (now removed)
- The resulting example should clearly demonstrate how `priority` is used to mark a fallback entry

### 2b. Add inline-table post-processing for providers and tools

Add a new function (e.g. `inline_dispatcher_tables`) in `init.rs` alongside the existing `inline_stage_tables`. This function:
- Navigates to `dispatcher.providers` (a TOML table) and converts each value entry to an inline table
- Navigates to `dispatcher.tools` (a TOML table of arrays) and converts each array's items to inline values

Follow the same `toml_edit` pattern as `inline_stage_tables`: collect keys, clone each item as `InlineTable`, replace the item, optionally call `set_dotted(true)` on the parent.

Call this new function from the same place `inline_stage_tables` is called (the `write_config_file` or equivalent function that serializes the config).

**Why:** Currently providers and tools are written as multi-line TOML tables, which is verbose. Compact inline form (`name = { executor = "..." }`) is more readable for the example config, matching the style shown in the task description.
