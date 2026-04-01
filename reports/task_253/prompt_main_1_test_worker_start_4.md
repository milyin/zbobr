Implement and run the tests described in the plans.

## Workflow

1. For each unchecked checklist item related to tests, implement the corresponding test. Commit your work after implementing each item.
2. Run only the tests mentioned in the checklist items, both checked and unchecked.
3. If tests fail, call `report_failure` and include failure details.
4. If tests pass, call `report_success`.

## Important
Do not implemet any functionality, your jond is only to implement and run tests.

---

# Current task: simplification to work on single repositiory

# Task description

In practice the typical use case for zbobr is to handle only one repository. To work on multiple project it's no problem to run multiple zbobr instances.
This allows to significantly simplify the code and workflow.
New simplified specifications:
- preparator stage is fully removed. The name of the branch is created in the code, just with task name
- the repo backend provides access to only one repository configured in it's config. It doesn't support multiple repositories
- the github repo backend don't care about "fork" concept of the github. It works inside repository only
- the config parameters changes:
  - dispatcher's `default_destination_repository`, `default_destination_branch` -> repo's `repository`, `branch` 
  - `fork_owner` - removed, no need to fork anymore

# Destination branch: main

# Work branch: zbobr_fix-253-simplify-single-repo

# Context

<!-- stage -->
- skynet:main:1:**preparing** `copilot` `gpt-5-mini` `2026-03-31 17:14:33 +0200`
  - ✅ Configured worktree for simplification to single repository <sub>[ctx_rec_1](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_preparing_report_success.md)</sub>
<!-- stage -->
- skynet:main:1:**planning** `claude` `claude-sonnet-4.6` `2026-03-31 17:17:03 +0200`
  - 💬 Plan ready for review: simplify to single-repo support <sub>[ctx_rec_2](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_planning_report_intermediate.md)</sub>
- user:**milyin** proceed with the plan `2026-03-31 15:34:24 +0000` <sub>[link](https://github.com/milyin/zbobr/issues/253#issuecomment-4163542594)</sub>
<!-- stage -->
- skynet:main:1:**planning** `claude` `claude-sonnet-4.6` `2026-03-31 17:45:25 +0200`
  - ✅ Plan finalized with 6 checklist items covering: remove preparator, auto-derive branch names, move repo/branch config to backend, remove fork support, simplify TaskIdentity, update tests/docs <sub>[ctx_rec_9](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_planning_report_success.md)</sub>
    - [x] Remove preparator stage from default workflow <sub>[ctx_rec_3](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_planning_item.md)</sub>
    - [x] Derive work branch name automatically from task name <sub>[ctx_rec_4](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_planning_item_1.md)</sub>
    - [x] Move repository/branch from dispatcher config to repo backend config <sub>[ctx_rec_5](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_planning_item_2.md)</sub>
    - [x] Remove fork support from GitHub repo backend <sub>[ctx_rec_6](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_planning_item_3.md)</sub>
    - [x] Simplify TaskIdentity and repo backend trait interface <sub>[ctx_rec_7](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_planning_item_4.md)</sub>
    - [x] Update tests, config examples, and documentation <sub>[ctx_rec_8](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_planning_item_5.md)</sub>
<!-- stage -->
- skynet:main:1:**working** `claude` `claude-sonnet-4.6` `2026-03-31 17:52:10 +0200`
<!-- stage -->
- skynet:main:1:**working** `claude` `claude-sonnet-4.6` `2026-03-31 18:08:26 +0200`
<!-- stage -->
- skynet:main:1:**working** `claude` `default` `2026-03-31 23:41:54 +0200`
<!-- stage -->
- skynet:main:1:**working** `claude` `claude-sonnet-4.6` `2026-03-31 23:46:44 +0200`
  - ✅ Simplified zbobr to single-repo support: removed preparator stage, configure_worktree tool, fork support, and per-task routing config. Work branch is now auto-derived from task title. <sub>[ctx_rec_10](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_working_report_success.md)</sub>
<!-- stage -->
- skynet:main:1:**test_planner** `claude` `claude-opus-4.6` `2026-04-01 00:24:35 +0200`
  - ✅ Test plan ready: 4 checklist items covering sanitize_branch_postfix, repo_short_name, simplified TaskIdentity, and preparator removal assertion <sub>[ctx_rec_15](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_test_planner_report_success.md)</sub>
    - [x] Add unit tests for `sanitize_branch_postfix()` in `zbobr-dispatcher/src/cli.rs` <sub>[ctx_rec_11](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_test_planner_item.md)</sub>
    - [x] Add unit tests for `repo_short_name()` on both FS and GitHub backend configs <sub>[ctx_rec_12](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_test_planner_item_1.md)</sub>
    - [x] Add unit test for simplified `TaskIdentity` — `identity()` only needs `work_branch` <sub>[ctx_rec_13](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_test_planner_item_2.md)</sub>
    - [x] Extend `default_workflow_includes_test_stages` test to assert preparator stage/role removed <sub>[ctx_rec_14](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_test_planner_item_3.md)</sub>
<!-- stage -->
- skynet:main:1:**test_worker** `claude` `claude-opus-4.6` `2026-04-01 00:29:38 +0200`
<!-- stage -->
- skynet:main:1:**test_worker** `claude` `claude-opus-4.6` `2026-04-01 01:17:56 +0200`
  - ✅ All 4 test checklist items implemented and passing: sanitize_branch_postfix (10 tests), repo_short_name FS+GitHub (9 tests), TaskIdentity identity (2 tests), preparator removal assertion (2 tests) — 23 tests total, all green <sub>[ctx_rec_16](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_test_worker_report_success.md)</sub>
<!-- stage -->
- skynet:main:1:**reviewing** `copilot` `gpt-5.4` `2026-04-01 01:25:54 +0200`
  - ❌ Review found 3 issues: Unicode panic in branch sanitization, leftover per-task routing causing inconsistent single-repo behavior, and an incorrect preparator-removal test. <sub>[ctx_rec_17](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_reviewing_report_failure.md)</sub>
<!-- stage -->
- skynet:main:1:**working** `claude` `claude-sonnet-4.6` `2026-04-01 01:30:39 +0200`
  - ✅ Fixed all 3 issues from code review (ctx_rec_17): Unicode panic in sanitize_branch_postfix (char-based truncation), removed per-task destination_repository/destination_branch fields from Task struct and all callers, fixed preparator test to check stage key "preparing" and role field. <sub>[ctx_rec_21](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_working_report_success_1.md)</sub>
    - [x] Fix Unicode panic in sanitize_branch_postfix() - use char-based truncation <sub>[ctx_rec_18](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item.md)</sub>
    - [x] Remove destination_repository/destination_branch from Task struct and fix finish() to use repo_backend <sub>[ctx_rec_19](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_1.md)</sub>
    - [x] Fix incorrect preparator-removal test - check "preparing" not "preparator" stage name <sub>[ctx_rec_20](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_2.md)</sub>
<!-- stage -->
- skynet:main:1:**test_planner** `claude` `claude-opus-4.6` `2026-04-01 02:04:31 +0200`
  - ✅ No additional tests needed — existing 23 tests adequately cover all implementation changes (sanitize_branch_postfix, repo_short_name, TaskIdentity, preparator removal, integration test updates). Pre-existing test failure in compact_comment_uses_first_line_only is unrelated to this branch. <sub>[ctx_rec_22](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_test_planner_report_success_1.md)</sub>
<!-- stage -->
- skynet:main:1:**test_worker** `claude` `claude-opus-4.6` `2026-04-01 02:08:15 +0200`
<!-- stage -->
- skynet:main:1:**test_worker** `claude` `claude-opus-4.6` `2026-04-01 04:25:43 +0200`
  - ❌ 22 tests pass but zbobr crate fails to compile — commands.rs still references removed VAR_DESTINATION_BRANCH, VAR_DESTINATION_REPOSITORY, and Task fields destination_branch/destination_repository <sub>[ctx_rec_23](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_test_worker_report_failure.md)</sub>
<!-- stage -->
- skynet:main:1:**working** `claude` `claude-sonnet-4.6` `2026-04-01 04:28:12 +0200`
  - ✅ Fixed zbobr crate compilation: re-exported VAR_DESTINATION_BRANCH/VAR_DESTINATION_REPOSITORY from zbobr_dispatcher and added missing Task fields to dummy_task_and_comments(). All 46 tests pass (1 pre-existing unrelated failure remains). <sub>[ctx_rec_25](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_working_report_success_2.md)</sub>
    - [x] Fix compilation errors: re-export VAR_DESTINATION_* constants from dispatcher and add missing Task fields to dummy_task_and_comments() <sub>[ctx_rec_24](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_3.md)</sub>
<!-- stage -->
- skynet:main:1:**test_planner** `claude` `claude-opus-4.6` `2026-04-01 04:31:36 +0200`
  - ✅ No additional tests needed — latest fix (73ad21d) was purely a compilation fix (re-export constants, add None fields to test helper). All 46 tests pass; 1 pre-existing unrelated failure (compact_comment_uses_first_line_only) remains. <sub>[ctx_rec_26](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_test_planner_report_success_2.md)</sub>
<!-- stage -->
- skynet:main:1:**test_worker** `claude` `claude-opus-4.6` `2026-04-01 04:33:41 +0200`
