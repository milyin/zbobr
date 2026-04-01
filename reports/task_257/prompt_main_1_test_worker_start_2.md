Implement the requested tests and run them.

## Workflow

1. For each unchecked checklist item related to tests, implement the corresponding test. Commit your work after implementing each item.
2. Run the implemented tests.
3. If tests fail, call `report_failure` and include failure details.
4. If tests pass, call `report_success`.

## Important
Do not implement any functionality, your job is only to implement and run tests according to the unchecked checklist items.

---

# Current task: remove noice from context for prompt

# Task description

The stage detatils (pipeline, timestamp, model, prompl link, output) just adds useless noice to the context. For the prompt provide the context in form, 

- planning
  - 💬 Plan ready for review: bla-bla-bla [ctx_rec_2]
- user milyin: proceed with the plan
- planning
  - ✅ Plan finalized bla bla bla [ctx_rec_9]
    - [x] plan item [ctx_rec_3]

I.e. 
- remove all stage information except stage name
- remove unnecessary formatting and links
- remove context links. To get linked context data provide mcp mehod `get_ctx_rec` instead (similar to delete_ctx_rec) which returns file content referenced by the link of corresponding ctx_rec_X.

# Destination branch: main

# Work branch: zbobr_fix-257-remove-noice-from-context-for-prompt

# Context

<!-- stage -->
- skynet:main:1:**planning** `claude` `claude-sonnet-4.6` `2026-04-01 16:46:50 +0200`
  - 💬 Plan ready for review: simplify prompt context rendering (stage name only, [ctx_rec_N] format, clean comments) and add get_ctx_rec MCP tool <sub>[ctx_rec_1](https://github.com/milyin/zbobr/blob/reports/reports/task_257/report_main_1_planning_report_intermediate.md)</sub>
- user:**milyin** One additional requirement: when stage contains no subitems (usually failed stage), filter it out `2026-04-01 14:48:12 +0000` <sub>[link](https://github.com/milyin/zbobr/issues/257#issuecomment-4170632254)</sub>
- user:**milyin** add requirement above and start implementing the plan `2026-04-01 14:54:53 +0000` <sub>[link](https://github.com/milyin/zbobr/issues/257#issuecomment-4170679232)</sub>
<!-- stage -->
- skynet:main:1:**planning** `claude` `claude-sonnet-4.6` `2026-04-01 16:56:42 +0200`
  - 💬 Plan: simplify for-prompt context rendering (stage name only, plain [ctx_rec_N], no timestamps/links, filter empty stages) and add get_ctx_rec MCP tool <sub>[ctx_rec_2](https://github.com/milyin/zbobr/blob/reports/reports/task_257/report_main_1_planning_report_intermediate_1.md)</sub>
- user:**milyin** Proceed with the plan `2026-04-01 16:05:36 +0000` <sub>[link](https://github.com/milyin/zbobr/issues/257#issuecomment-4171147703)</sub>
<!-- stage -->
- skynet:main:1:**planning** `claude` `claude-sonnet-4.6` `2026-04-01 18:07:17 +0200`
  - ✅ Plan finalized: simplify for-prompt context rendering (stage name only, plain [ctx_rec_N], no timestamps/links in comments, filter empty stages) and add get_ctx_rec MCP tool <sub>[ctx_rec_5](https://github.com/milyin/zbobr/blob/reports/reports/task_257/report_main_1_planning_report_success.md)</sub>
    - [x] Simplify for-prompt context rendering in zbobr-api <sub>[ctx_rec_3](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_planning_item.md)</sub>
    - [x] Add get_ctx_rec MCP tool <sub>[ctx_rec_4](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_planning_item_1.md)</sub>
<!-- stage -->
- skynet:main:1:**working** `claude` `claude-sonnet-4.6` `2026-04-01 18:13:19 +0200`
<!-- stage -->
- skynet:main:1:**working** `copilot` `claude-sonnet-4.6` `2026-04-01 19:10:00 +0200`
  - ✅ Implemented simplified for-prompt context rendering and get_ctx_rec MCP tool <sub>[ctx_rec_8](https://github.com/milyin/zbobr/blob/reports/reports/task_257/report_main_1_working_report_success.md)</sub>
    - [ ] Simplify for-prompt context rendering in zbobr-api <sub>[ctx_rec_6](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_working_item.md)</sub>
    - [x] Add get_ctx_rec MCP tool <sub>[ctx_rec_7](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_working_item_1.md)</sub>
<!-- stage -->
- skynet:main:1:**test_planner** `copilot` `claude-opus-4.6` `2026-04-01 19:23:29 +0200`
  - ✅ Test plan: 5 test groups identified — 2 high priority (empty stage filtering, get_context_record_content), 2 medium (MCP integration, parse_ctx_rec_id), 1 low (Display unit tests) <sub>[ctx_rec_10](https://github.com/milyin/zbobr/blob/reports/reports/task_257/report_main_1_test_planner_report_success.md)</sub>
    - [x] Test: MdRecord, MdCompactComment, MdStage Display with for_prompt=true <sub>[ctx_rec_9](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_test_planner_item.md)</sub>
    - [x] Test: get_context_record_content returns report content, brief, or None <sub>[ctx_rec_11](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_test_planner_item_1.md)</sub>
    - [x] Test: get_ctx_rec MCP tool integration (end-to-end) <sub>[ctx_rec_12](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_test_planner_item_2.md)</sub>
    - [x] Test: empty stages filtered out in for_prompt mode <sub>[ctx_rec_13](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_test_planner_item_3.md)</sub>
    - [x] Test: parse_ctx_rec_id unit tests for numeric, prefixed, and error cases <sub>[ctx_rec_14](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_test_planner_item_4.md)</sub>
<!-- stage -->
- skynet:main:1:**test_worker** `copilot` `claude-opus-4.6` `2026-04-01 19:28:40 +0200`
  - ✅ All 5 test groups implemented and passing: 11 new tests across 3 files <sub>[ctx_rec_15](https://github.com/milyin/zbobr/blob/reports/reports/task_257/report_main_1_test_worker_report_success.md)</sub>
<!-- stage -->
- skynet:main:1:**reviewing** `copilot` `gpt-5.4` `2026-04-01 19:36:44 +0200`
  - ❌ Review found 2 blocking issues: prompt comment format still doesn’t match the requested simplified form, and the new get_ctx_rec tool was not added to the MCP integration test surface. <sub>[ctx_rec_16](https://github.com/milyin/zbobr/blob/reports/reports/task_257/report_main_1_reviewing_report_failure.md)</sub>
<!-- stage -->
- skynet:main:1:**working** `copilot` `claude-sonnet-4.6` `2026-04-01 19:40:55 +0200`
  - ✅ Fixed both review blocking issues: plain comment format and get_ctx_rec MCP integration test <sub>[ctx_rec_19](https://github.com/milyin/zbobr/blob/reports/reports/task_257/report_main_1_working_report_success_1.md)</sub>
    - [ ] Add get_ctx_rec step to MCP integration test scenario <sub>[ctx_rec_17](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_working_item_2.md)</sub>
    - [x] Fix comment format: remove bold markdown (user:**name** -> user name:) <sub>[ctx_rec_18](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_working_item_3.md)</sub>
<!-- stage -->
- skynet:main:1:**test_planner** `copilot` `claude-opus-4.6` `2026-04-01 19:47:35 +0200`
  - ✅ No additional tests needed — all 5 test groups from round 1 are implemented and passing (17 new tests). Coverage is comprehensive. <sub>[ctx_rec_20](https://github.com/milyin/zbobr/blob/reports/reports/task_257/report_main_1_test_planner_report_success_1.md)</sub>
<!-- stage -->
- skynet:main:1:**test_worker** `copilot` `claude-opus-4.6` `2026-04-01 19:51:49 +0200`
  - ✅ All tests passing — 57 lib tests + 13 integration tests. The unchecked test item (ctx_rec_17: get_ctx_rec MCP integration step) was already implemented in commit 534cb58. No additional test work required. <sub>[ctx_rec_21](https://github.com/milyin/zbobr/blob/reports/reports/task_257/report_main_1_test_worker_report_success_1.md)</sub>
<!-- stage -->
- skynet:main:1:**reviewing** `copilot` `gpt-5.4` `2026-04-01 19:57:39 +0200`
  - ❌ Review failed: prompt output still includes stage markers, and prompt-specific comment cleanup regressed normal context formatting. <sub>[ctx_rec_22](https://github.com/milyin/zbobr/blob/reports/reports/task_257/report_main_1_reviewing_report_failure_1.md)</sub>
<!-- stage -->
- skynet:main:1:**working** `copilot` `claude-sonnet-4.6` `2026-04-01 20:01:26 +0200`
  - ✅ Fixed both review blocking issues: stage markers now gated to non-prompt mode, non-prompt comment format restored to user:**name** bold style <sub>[ctx_rec_25](https://github.com/milyin/zbobr/blob/reports/reports/task_257/report_main_1_working_report_success_2.md)</sub>
    - [x] Fix MdContext::fmt to not emit stage markers in for_prompt mode <sub>[ctx_rec_23](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_working_item_4.md)</sub>
    - [x] Fix MdCompactComment::from_comment to preserve non-prompt format (user:**name** bold, join lines with spaces) <sub>[ctx_rec_24](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_working_item_5.md)</sub>
<!-- stage -->
- skynet:main:1:**test_planner** `copilot` `claude-opus-4.6` `2026-04-01 20:09:48 +0200`
  - ✅ Test plan: 2 new tests identified — 1 high priority (end-to-end prompt format), 1 medium (multi-line comment in prompt mode). Existing 17 tests provide good component-level coverage. <sub>[ctx_rec_28](https://github.com/milyin/zbobr/blob/reports/reports/task_257/report_main_1_test_planner_report_success_2.md)</sub>
    - [ ] Test: end-to-end prompt format validation (stages + comments + empty stage filtering) <sub>[ctx_rec_26](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_test_planner_item_5.md)</sub>
    - [ ] Test: multi-line comment body preserved in for_prompt mode <sub>[ctx_rec_27](https://github.com/milyin/zbobr/blob/reports/reports/task_257/checklist_main_1_test_planner_item_6.md)</sub>
<!-- stage -->
- skynet:main:1:**test_worker** `copilot` `claude-opus-4.6` `2026-04-01 20:17:26 +0200`
