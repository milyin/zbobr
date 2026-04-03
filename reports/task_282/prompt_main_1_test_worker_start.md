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

# Current task: reuse sample task code

# Task description

There is function "dummy_task_and_comments" function. Do this:
- rename "dummy_task_and_comments" to "sample_task_and_comments"
- fill url fields, context, signal, stack with some non-trivial values
- use it for validation made in #266 

# Destination branch: main

# Work branch: zbobr_fix-282-reuse-sample-task-code

# Context

- planning
  - 💬 Plan ready for review [ctx_rec_1]
- user milyin: proceed with the plan
- planning
  - ✅ Plan approved and checklist items created [ctx_rec_4]
    - [x] Move and rename dummy_task_and_comments to sample_task_and_comments in zbobr-dispatcher [ctx_rec_2]
    - [x] Update validate_all_prompts to use sample_task_and_comments [ctx_rec_3]
- working
  - ✅ Renamed dummy_task_and_comments to sample_task_and_comments with non-trivial values; updated validate_all_prompts to use it [ctx_rec_5]
- reviewing
  - ❌ Review found a standards issue in sample_task_and_comments: newly introduced canonical strings are hardcoded instead of reused from existing constants. [ctx_rec_6]
- working
  - ✅ Fixed canonical string issues in sample_task_and_comments: replaced hardcoded "claude" with Tool::CLAUDE, factored repeated URL prefix into local consts [ctx_rec_8]
    - [x] Fix hardcoded canonical strings in sample_task_and_comments: use Tool::CLAUDE constant and factor URL prefix [ctx_rec_7]
- reviewing
  - ✅ Review passed: sample_task_and_comments refactor is consistent with the plan, fixes the prior canonical-string issue, and no further issues were found. [ctx_rec_9]
- test_planner
  - ✅ Test plan complete: one unit test needed for sample_task_and_comments() [ctx_rec_11]
    - [ ] Add unit test for sample_task_and_comments() asserting non-trivial field values [ctx_rec_10]
