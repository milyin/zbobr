## Plan Summary

### Approach
The plan follows two existing analogs:
1. `ProviderDefinition.priority: Option<i32>` — same pattern for the new `ToolEntry.priority` field
2. `inline_stage_tables()` in `zbobr/src/init.rs` — same `toml_edit` approach for compacting provider/tool tables

### Key Design Decisions
- Per-entry priority uses `entry.priority.unwrap_or(rp.priority)` — clean fallback with no behavior change when field is absent
- The `copilot_planner` provider is removed from the example (not needed; `claude_planner` is sufficient to demonstrate the `plan_mode` pattern)
- Tools are renamed to match the task description's example (`developer` instead of `smart`)
- Inline-table formatting is a pure presentation change — no semantic impact on config loading

### Checklist Items
1. Add `priority: Option<i32>` to `ToolEntry` struct in `zbobr-api/src/config.rs`
2. Update `select_provider_excluding` in `zbobr-dispatcher/src/lib.rs` to use entry priority override; update test helper and add new test
3. Update `default_config_toml()` in `zbobr/src/init.rs`: simplify providers/tools, add priority example, add `inline_dispatcher_tables()` post-processing function
