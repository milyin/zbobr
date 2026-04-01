Implement the requested tests and run them.

## Workflow

1. For each unchecked checklist item related to tests, implement the corresponding test. Commit your work after implementing each item.
2. Run the implemented tests.
3. If tests fail, call `report_failure` and include failure details.
4. If tests pass, call `report_success`.

## Important
Do not implement any functionality, your job is only to implement and run tests according to the unchecked checklist items.

---

# Current task: remove DeleteCtxRec action

# Task description

Disallow agents to delete records from context: remove DeleteCtxRec operation.
Do not print in the prompt mode `ctx_rec_{}` to records without links or checkboxes. The agent don't need the ids of the records which can't be interacted (with get content or check item operations).

# Destination branch: main

# Work branch: zbobr_fix-263-remove-deletectxrec-action

# Context

- user milyin: Verify how missing ctx_rec will affect context parsing. Consider making this field optional
- planning
  - 💬 Plan ready for review: remove DeleteCtxRec tool and suppress ctx_rec IDs in prompt mode for non-interactive records [ctx_rec_1]
- user milyin: proceed with the plan
- planning
  - ✅ Plan ready: remove DeleteCtxRec tool and suppress ctx_rec IDs for non-interactive records in prompt mode [ctx_rec_5]
    - [x] Remove DeleteCtxRec from McpTool enum and role configs [ctx_rec_2]
    - [x] Remove delete_ctx_rec MCP handler and supporting code [ctx_rec_3]
    - [x] Suppress ctx_rec IDs for non-interactive records in prompt mode [ctx_rec_4]
- working
  - ✅ Removed DeleteCtxRec MCP tool and suppressed non-interactive ctx_rec IDs in prompt mode. All tests pass. [ctx_rec_6]
- test_planner
  - ✅ Test plan ready: 3 checklist items covering unit tests for non-interactive ID suppression, strengthening existing tests with negative assertions, and an end-to-end mixed-record test. [ctx_rec_10]
    - [ ] Strengthen existing prompt-mode tests with assertions for non-interactive ID absence [ctx_rec_7]
    - [ ] Add unit tests for MdRecord non-interactive ID suppression in prompt mode [ctx_rec_8]
    - [ ] Add end-to-end test with mixed interactive and non-interactive records in prompt mode [ctx_rec_9]
