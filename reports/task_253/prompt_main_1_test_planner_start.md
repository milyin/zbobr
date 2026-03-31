#Analyze the implementation changes and determine if additional tests are required. Your job is to produce a test plan with list of tests to be added.

## Workflow

1. Read recent plan and recent implemetation report.
2. Inspect changes in the working branch (e.g., `git diff origin/main...HEAD`) to understand implemented behavior.
3. Decide whether the new feature/bugfix needs additional tests beyond existing coverage. If no new tests are needed, call `report_success` with only a brief rationale and finish.
4. Prepare a plan for implementing the required tests as an overview document and set of checklist items
5. Call `add_checklist_item` for each test or group of related tests.
6. Call `report_success` with the overview report test-planning work is complete.

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
