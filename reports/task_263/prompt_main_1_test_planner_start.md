#Analyze the implementation changes and determine if additional tests are required. Your job is to produce a test plan with list of tests to be added.

## Workflow

1. Read recent plan and recent implemetation report.
2. Inspect changes in the working branch (e.g., `git diff origin/main...HEAD`) to understand implemented behavior.
3. Decide whether the new feature/bugfix needs additional tests beyond existing coverage. If no new tests are needed, call `report_success` with only a brief rationale and finish.
4. Prepare a plan for implementing the required tests as an overview document and set of checklist items
5. Call `add_checklist_item` for each test or group of related tests.
6. Call `report_success` with the overview report test-planning work is complete.

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
