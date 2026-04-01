# Worker Agent

Implement the task accordingly to the final plan in the context. Notice that there can be multiple plan versions in the history, work on the last one. If the plan is accompanied by checklist items, process them one by one, skip the checked ones. If there are no checklst items, analyze the pan and create checklist items for the implementation steps yourself.

- Use `check_checklist_item` to mark item as done when you complete the subtask in it.
- Use `add_checklist_item` to add new item when you discover new job to do or user made additional request in comments.

## Access Model

You can access the internet and run local commands. Your restrictions:
- Do NOT push code directly — no `git push`, no `gh` write operations. The platform coordinates repository remote actions; do not include submission or remote-write actions as checklist items.
- Do NOT run git clone/pull/fetch — your current working directory is already the repository with the work branch checked out.
- For reading GitHub data: use `git` and `gh` CLI only when no platform tool provides the needed information.
- NEVER use git/gh for writing, pushing, or sending data to GitHub.
- The work repository has remote information controlled by the platform; you must not perform direct remote writes yourself.

## Workspace isolation

Workspace branch isolation. Your working directory is already the repository with the work branch checked out. Do not make changes in the destination branch: this is for reference only. Do NOT fetch or use any other branches. If you need temporary or experimental branches, prefix their names with the work branch name to avoid interfering with other agents.

Work autonomously. Do not ask the user for anything unless the task genuinely requires human input.

## Workflow

1. Read the task description, context, and comments provided below in this prompt. The full history and checklist are available in the context section.
2. **Identify the analog referenced in the plan.** Before writing any code, study the analogous existing code mentioned by the planner. Your implementation MUST follow the same patterns, conventions, coding style, and architectural approaches as the analog. If no analog is mentioned, search for similar functionality in the codebase yourself before proceeding.
3. Implement the task by going through unchecked checklist items one by one. Commit work after implementing each item.  **Follow the same patterns and style as the identified analog if one is available.**
4. When implementation for an item is complete, mark the item done with `check_checklist_item` (pass the ctx_rec_N id).
5. Correct existing tests if necessary, but **do NOT implement new tests for new functionality** in this stage. Tests will be implemented later.
6. If you sense your context window is getting close to its limit, finish your current item to a buildable state, commit your work, mark completed items as done, call `report_intermediate` with a summary of what you accomplished and what remains and finish the session.
7. If you need human clarification or intervention, call `stop_with_question`. If the plan is unclear or requires adjustment, call `report_failure`. In case of technical errors use `stop_with_error`.
8. If some instrument is required and you can't install it yourself, ask the user to install it with `stop_with_question`.
9. When your current session's work is done, decide how to finish:
    - If **all checklist items are completed** (the full plan is done), call `report_success` to report final success.
    - If **some items remain unchecked** (more work is needed in future sessions), call `report_intermediate` to report what you accomplished so far.

## Coding Guidelines

- **Prefer deriving values from types and constants** rather than using hardcoded string literals. If a value can be computed from an existing type, enum variant, or constant, do it. Avoid duplicating the value as literals or constants.

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
  - ✅ All 24 tests pass across all 4 checklist areas: sanitize_branch_postfix (11), repo_short_name FS+GitHub (9), TaskIdentity identity (2), preparator removal assertion (2). No new tests needed — all items were already implemented. <sub>[ctx_rec_27](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_test_worker_report_success_1.md)</sub>
<!-- stage -->
- skynet:main:1:**reviewing** `copilot` `gpt-5.4` `2026-04-01 04:36:08 +0200`
  - ❌ Review failed: single-repo simplification is incomplete because task-level destination repo/branch remain authoritative in dispatcher code. <sub>[ctx_rec_28](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_reviewing_report_failure_1.md)</sub>
<!-- stage -->
- skynet:main:1:**working** `claude` `claude-sonnet-4.6` `2026-04-01 04:39:56 +0200`
  - ✅ Removed destination_repository/destination_branch from Task struct and all callers. The repo backend is now the sole source of truth for routing config. All 46 tests pass (1 pre-existing unrelated failure remains). <sub>[ctx_rec_32](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_working_report_success_3.md)</sub>
    - [x] Remove destination_repository/destination_branch fields from Task struct in zbobr-api/src/task.rs <sub>[ctx_rec_29](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_4.md)</sub>
    - [x] Remove destination_repository/destination_branch from github backend (issue_to_task, task_to_string_params) <sub>[ctx_rec_30](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_5.md)</sub>
    - [x] Remove destination_repository/destination_branch from dispatcher (cli.rs, task.rs, prompts.rs, workflow.rs, commands.rs) <sub>[ctx_rec_31](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_6.md)</sub>
<!-- stage -->
- skynet:main:1:**test_planner** `claude` `claude-opus-4.6` `2026-04-01 05:11:34 +0200`
  - ✅ No additional tests needed — latest fix (93378bb) only removed destination_repository/destination_branch fields from Task struct and all callers. All 59+ existing tests pass and already cover the simplified single-repo behavior. 1 pre-existing unrelated failure (compact_comment_uses_first_line_only) remains. <sub>[ctx_rec_33](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_test_planner_report_success_3.md)</sub>
<!-- stage -->
- skynet:main:1:**test_worker** `claude` `claude-opus-4.6` `2026-04-01 05:15:01 +0200`
  - ✅ All 23 checklist tests pass: sanitize_branch_postfix (11), repo_short_name FS+GitHub (9), TaskIdentity identity (2), preparator removal (1). No new tests needed — all items were already implemented. <sub>[ctx_rec_34](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_test_worker_report_success_2.md)</sub>
<!-- stage -->
- skynet:main:1:**reviewing** `copilot` `gpt-5.4` `2026-04-01 05:17:34 +0200`
  - ❌ Review failed: found 3 issues in single-repo simplification (GitHub repo name normalization bug, missing branch validation, outdated docs/examples). <sub>[ctx_rec_35](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_reviewing_report_failure_2.md)</sub>
<!-- stage -->
- skynet:main:1:**working** `claude` `claude-sonnet-4.6` `2026-04-01 05:22:27 +0200`
  - ✅ Fixed all 3 issues from review (ctx_rec_35): GitHub repo_short_name URL normalization, branch validation in both backends, and outdated docs. 46 tests pass (1 pre-existing unrelated failure remains). <sub>[ctx_rec_39](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_working_report_success_4.md)</sub>
    - [x] Fix GitHub backend repo_short_name() to normalize URLs (strip .git suffix, trailing slashes) <sub>[ctx_rec_36](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_7.md)</sub>
    - [x] Add branch field validation to both FS and GitHub backend config validators <sub>[ctx_rec_37](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_8.md)</sub>
    - [x] Update README.md and docs to remove preparator, fork_owner references <sub>[ctx_rec_38](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_9.md)</sub>
<!-- stage -->
- skynet:main:1:**test_planner** `claude` `claude-opus-4.6` `2026-04-01 05:29:18 +0200`
  - ✅ Test plan: 1 checklist item — add validate() unit tests for branch/repository empty checks on both FS and GitHub backend configs (5 tests total) <sub>[ctx_rec_41](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_test_planner_report_success_4.md)</sub>
    - [x] Add unit tests for config `validate()` on both FS and GitHub backends — branch-empty and repository-empty cases <sub>[ctx_rec_40](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_test_planner_item_4.md)</sub>
<!-- stage -->
- skynet:main:1:**test_worker** `claude` `claude-opus-4.6` `2026-04-01 05:33:07 +0200`
  - ✅ All 34 checklist tests pass: sanitize_branch_postfix (11), FS config repo_short_name+validate (9), GitHub config repo_short_name+validate (11), TaskIdentity (2), preparator removal (1). New validate() tests added for ctx_rec_40. <sub>[ctx_rec_42](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_test_worker_report_success_3.md)</sub>
<!-- stage -->
- skynet:main:1:**reviewing** `copilot` `gpt-5.4` `2026-04-01 05:51:18 +0200`
  - ❌ Review failed: found 3 remaining issues in single-repo simplification (GitHub repo normalization, wrong GitHub test target wiring, outdated docs/examples). <sub>[ctx_rec_43](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_reviewing_report_failure_3.md)</sub>
<!-- stage -->
- skynet:main:1:**working** `claude` `claude-sonnet-4.6` `2026-04-01 05:56:51 +0200`
  - ✅ Fixed all 3 issues from review (ctx_rec_43): GitHub repository normalization in from_config() (HTTPS/SSH URLs now normalized to owner/repo), wrong target_repo wiring in init_github_github (was task_repo, now repository), and outdated fork-owner reference in README.md. All 46 tests pass (1 pre-existing unrelated failure remains). <sub>[ctx_rec_47](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_working_report_success_5.md)</sub>
    - [x] Normalize backend_config.repository to owner/repo in from_config() to fix API calls <sub>[ctx_rec_44](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_10.md)</sub>
    - [x] Fix target_repo wiring in init_github_github: use repository instead of task_repo <sub>[ctx_rec_45](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_11.md)</sub>
    - [x] Fix outdated docs: remove (fork owner, default model) from README.md <sub>[ctx_rec_46](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_12.md)</sub>
<!-- stage -->
- skynet:main:1:**test_planner** `claude` `claude-opus-4.6` `2026-04-01 06:08:43 +0200`
<!-- stage -->
- skynet:main:1:**test_planner** `claude` `claude-opus-4.6` `2026-04-01 10:15:56 +0200`
  - ✅ Test plan: 2 checklist items — add unit tests for parse_github_repo() (9 cases) and from_config() normalization (2 cases), covering the repository normalization logic added in latest fixes <sub>[ctx_rec_50](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_test_planner_report_success_5.md)</sub>
    - [x] Add unit tests for `parse_github_repo()` in `zbobr-repo-backend-github/src/github.rs` <sub>[ctx_rec_48](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_test_planner_item_5.md)</sub>
    - [x] Add test verifying `from_config()` normalizes repository field to `owner/repo` format <sub>[ctx_rec_49](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_test_planner_item_6.md)</sub>
<!-- stage -->
- skynet:main:1:**test_worker** `claude` `claude-opus-4.6` `2026-04-01 10:19:28 +0200`
  - ✅ All 45 checklist tests pass: implemented parse_github_repo (8 tests) and from_config normalization (2 tests) for ctx_rec_48/ctx_rec_49. Prior checklist items (sanitize_branch_postfix 11, repo_short_name FS 9, GitHub config 11, TaskIdentity 2, preparator removal 2) all green. <sub>[ctx_rec_51](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_test_worker_report_success_4.md)</sub>
<!-- stage -->
- skynet:main:1:**reviewing** `copilot` `gpt-5.4` `2026-04-01 10:24:24 +0200`
  - ❌ Review failed: found 3 remaining issues — prompt preview regression, leftover multi-repo lookup in FS backend, and still-outdated single-repo docs/CLI examples. <sub>[ctx_rec_52](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_reviewing_report_failure_4.md)</sub>
<!-- stage -->
- skynet:main:1:**working** `claude` `claude-sonnet-4.6` `2026-04-01 10:29:16 +0200`
  - ✅ Fixed all 3 issues from review (ctx_rec_52): dry-run prompt path now populates VAR_DESTINATION_REPOSITORY/BRANCH from repo config, FS ensure_pr_url() constrained to configured repo's bare clone, README updated to remove obsolete commands (task plan/work, --tasks-github-task-repo flags). 46 tests pass (1 pre-existing unrelated failure remains). <sub>[ctx_rec_56](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_working_report_success_6.md)</sub>
    - [x] Fix dry-run path: ensure prompt variables are populated from repo config even when no backend is active <sub>[ctx_rec_53](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_13.md)</sub>
    - [x] Fix FS backend ensure_pr_url() to only look at the configured repository's bare clone, not all .git entries under repos_dir <sub>[ctx_rec_54](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_14.md)</sub>
    - [x] Fix README docs: remove obsolete commands (zbobr setup --tasks-github-task-repo, zbobr task plan) and update backend docs <sub>[ctx_rec_55](https://github.com/milyin/zbobr/blob/reports/reports/task_253/checklist_main_1_working_item_15.md)</sub>
<!-- stage -->
- skynet:main:1:**test_planner** `claude` `claude-opus-4.6` `2026-04-01 10:35:13 +0200`
  - ✅ No additional tests needed — latest fix (44e9ca3) contains only a docs update, a simplification of FS ensure_pr_url (already covered by integration tests), and a dry-run prompt var wiring fix (top-level dispatch, not practically unit-testable). All 172 existing tests cover the implemented behavior. <sub>[ctx_rec_57](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_test_planner_report_success_6.md)</sub>
<!-- stage -->
- skynet:main:1:**test_worker** `claude` `claude-opus-4.6` `2026-04-01 10:37:47 +0200`
  - ✅ All 45 checklist tests pass: sanitize_branch_postfix (11), repo_short_name FS+GitHub (12), TaskIdentity identity (2), preparator removal (2), FS validate (3), GitHub validate (5), parse_github_repo (8), from_config normalization (2). No new tests needed — all items were already implemented. <sub>[ctx_rec_58](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_test_worker_report_success_5.md)</sub>
<!-- stage -->
- skynet:main:1:**reviewing** `copilot` `gpt-5.4` `2026-04-01 10:40:51 +0200`
  - ❌ Review failed: found 2 must-fix issues — GitHub repo normalization still misses `.git/` URLs, and docs/examples remain inconsistent with the single-repo design. <sub>[ctx_rec_59](https://github.com/milyin/zbobr/blob/reports/reports/task_253/report_main_1_reviewing_report_failure_5.md)</sub>
<!-- stage -->
- skynet:main:1:**working** `claude` `claude-sonnet-4.6` `2026-04-01 10:49:25 +0200`
