# Test Planning Report

## Summary of Changes
Three source areas were modified:
1. **`zbobr-api/src/config.rs`** — added `priority: Option<i32>` with serde skip/default to `ToolEntry`
2. **`zbobr-dispatcher/src/lib.rs`** — `select_provider_excluding` now uses `entry.priority.unwrap_or(rp.priority)`; one new test added
3. **`zbobr/src/init.rs`** — new `inline_dispatcher_tables` function; default tool renamed from `smart` → `developer`; `copilot_planner` provider removed

## Existing Coverage
- The implementation already added `select_provider_entry_priority_overrides_provider` (priority-lowering direction)
- Config validation tests exist but don't cover serde for the new field
- `inline_stage_tables` has no tests; `inline_dispatcher_tables` follows the same pattern but is new and untested

## Required Tests (4 checklist items)

| # | Area | Description |
|---|------|-------------|
| ctx_rec_8 | `zbobr-api/src/config.rs` | Serde round-trip for `ToolEntry.priority`: deserialize with/without field, serialize None (skipped) and Some (included) |
| ctx_rec_9 | `zbobr/src/init.rs` | Unit tests for `inline_dispatcher_tables`: providers → inline tables, tools AOT → inline array, noop when missing, integration check on `default_config_toml` |
| ctx_rec_10 | `zbobr/src/init.rs` | Regression guard: default config has `"developer"` tool and no role references old `"smart"` name |
| ctx_rec_11 | `zbobr-dispatcher/src/lib.rs` | Entry priority elevation test (mirror of existing lowering test, confirms symmetry) |
