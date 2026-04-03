Implement the requested tests and run them.

- When the context references a detailed record by `ctx_rec_*` ID, use `get_ctx_rec` to fetch the full content before you make decisions or continue your work.


## Workflow

1. For each unchecked checklist item related to tests, implement the corresponding test. Commit your work after implementing each item.
2. Run the implemented tests.
3. If tests fail, call `report_failure` and include failure details.
4. If tests pass, call `report_success`.

## Important
Do not implement any functionality, your job is only to implement and run tests according to the unchecked checklist items.

---

# Current task: improve example config

# Task description

1. add `priority` field to the tool record. This value replaces priority inherited from provider
```
[dispatcher.tools]
developer = [
  { proviider = "claude", model = "claude-opus-4.6" },
  { proviider = "copilot", model = "claude-sonnet-4.6", priority = 0 } # resort to it only if claude fails
]
``` 
2. output providers and tools into example zbobr.toml on init stage in compacted form:

```
[dispatcher.providers]
copilot = { executor = "copilot" }
claude = { executor = "claude" }
claude_planner = { parent = "claude", plan_mode = true }

[dispatcher.tools]
developer = [
  { proviider = "claude", model = "claude-opus-4.6" },
  { proviider = "copilot", model = "claude-opus-4.6" }
]
```

# Destination branch: main

# Work branch: zbobr_fix-286-improve-example-config

# Context

- planning
  - 💬 Plan: add ToolEntry.priority field and improve init example config formatting [ctx_rec_1]
- user milyin: do the plan
- planning
  - ✅ Plan approved and checklist items created for: (1) add ToolEntry.priority field, (2) update dispatch logic, (3) update init example config with simplified providers and inline-table formatting [ctx_rec_5]
    - [x] Add `priority: Option<i32>` to `ToolEntry` in zbobr-api/src/config.rs [ctx_rec_2]
    - [x] Update dispatch priority logic to use per-entry priority override [ctx_rec_3]
    - [x] Update example config in init.rs: simplify providers, add priority example, inline-table formatting [ctx_rec_4]
- working
  - ✅ All three items implemented: ToolEntry.priority field added, dispatch logic updated, init example config improved with compact inline formatting [ctx_rec_6]
- reviewing
  - ✅ Review passed: implementation matches the plan and all checklist items appear complete. [ctx_rec_7]
- test_planner
  - ✅ Test plan complete: 4 checklist items covering serde round-trips for ToolEntry.priority, unit tests for inline_dispatcher_tables, a regression guard for the "developer" tool rename, and an elevation-direction test for entry priority overrides. [ctx_rec_12]
    - [ ] Add serde round-trip tests for ToolEntry.priority in zbobr-api/src/config.rs [ctx_rec_8]
    - [ ] Add unit tests for inline_dispatcher_tables in zbobr/src/init.rs [ctx_rec_9]
    - [ ] Add test verifying default config roles reference "developer" tool and it resolves correctly [ctx_rec_10]
    - [ ] Add dispatcher test: entry priority elevates an entry above its provider's default tier [ctx_rec_11]
