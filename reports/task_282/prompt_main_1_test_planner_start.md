#Analyze the implementation changes and determine if additional tests are required. Your job is to produce a test plan with list of tests to be added.

- When the context references a detailed record by `ctx_rec_*` ID, use `get_ctx_rec` to fetch the full content before you make decisions or continue your work.


## Workflow

1. Read recent plan and recent implemetation report.
2. Inspect changes in the working branch (e.g., `git diff origin/main...HEAD`) to understand implemented behavior.
3. Decide whether the new feature/bugfix needs additional tests beyond existing coverage. If no new tests are needed, call `report_success` with only a brief rationale and finish.
4. Prepare a plan for implementing the required tests as an overview document and set of checklist items
5. Call `add_checklist_item` for each test or group of related tests.
6. Call `report_success` with the overview report test-planning work is complete.

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
