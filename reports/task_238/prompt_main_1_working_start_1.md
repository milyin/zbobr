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
3. Implement the task by going through unchecked checklist items. Assume that checked items were completed in previous sessions. **Follow the same patterns and style as the identified analog if one is available.**
4. If you sense your context window is getting close to its limit, finish your current item to a buildable state, commit your work, mark completed items as done, call `report_intermediate` with a summary of what you accomplished and what remains and finish the session.
6. **Write tests for new functionality** unless explicitly specified to omit tests or the change is not code related (e.g., output messages, documentation updates, llm prompts) or the test is expected to be too complex or require specific environment. Tests should validate the added functionality.
7. Commit all your changes locally to the work branch with clear messages (describe what the change does, why, and reference relevant checklist item). ALWAYS ensure that you have no uncommitted changes before marking your checklist items as done.
8. When implementation for an item is complete, mark the item done with `check_checklist_item` (pass the ctx_rec_N id).
9. If you need human clarification or intervention, call `stop_with_question`. If the plan is unclear or requires adjustment, call `report_failure`. In case of technical errors use `stop_with_error`.
10. If some instrument is required and you can't install it yourself, ask the user to install it with `stop_with_question`.
11. When your current session's work is done, decide how to finish:
    - If **all checklist items are completed** (the full plan is done), call `report_success` to report final success.
    - If **some items remain unchecked** (more work is needed in future sessions), call `report_intermediate` to report what you accomplished so far.

## Coding Guidelines

- **Prefer deriving values from types and constants** rather than using hardcoded string literals. If a value can be computed from an existing type, enum variant, or constant, do it. Avoid duplicating the value as literals or constants.

---

# Current task: implement type for storing secrets

# Task description

store sensitive information in special type `Secret`. It's enum with currently 2 variants: `Value(secret_string)` and `Env(variable)`. Represent it in toml as either `{ value = "secret" }` or `{ env = "ENV_SECRET" }`
Do not keep backward compatibility, old, just string format. for token keys is not allowed anymore

# Destination branch: main

# Work branch: zbobr_fix-238-implement-secret-type

# Context

<!-- stage -->
- skynet:main:1:**preparing** `copilot` `gpt-5-mini` `2026-03-29 01:32:32 +0100`
<!-- stage -->
- skynet:main:1:**planning** `claude` `claude-sonnet-4.6` `2026-03-29 01:34:15 +0100`
  - 💬 Plan ready for review: introduce Secret enum in zbobr-api, migrate all 4 token fields <sub>[ctx_rec_1](https://github.com/milyin/zbobr/blob/reports/reports/task_238/report_main_1_planning_report_intermediate.md)</sub>
- user:**milyin** proceed with the plan `2026-03-29 00:40:26 +0000` <sub>[link](https://github.com/milyin/zbobr/issues/238#issuecomment-4149120879)</sub>
<!-- stage -->
- skynet:main:1:**planning** `claude` `claude-sonnet-4.6` `2026-03-29 01:44:28 +0100`
  - ✅ Plan approved and checklist ready: 4 items covering Secret type definition, field migration, callsite updates, and tests <sub>[ctx_rec_6](https://github.com/milyin/zbobr/blob/reports/reports/task_238/report_main_1_planning_report_success.md)</sub>
    - [x] Define `Secret` enum in zbobr-api <sub>[ctx_rec_2](https://github.com/milyin/zbobr/blob/reports/reports/task_238/checklist_main_1_planning_item.md)</sub>
    - [x] Migrate all 4 token fields from `String` to `Secret` <sub>[ctx_rec_3](https://github.com/milyin/zbobr/blob/reports/reports/task_238/checklist_main_1_planning_item_1.md)</sub>
    - [x] Update validation and callsites to use `Secret::resolve()` <sub>[ctx_rec_4](https://github.com/milyin/zbobr/blob/reports/reports/task_238/checklist_main_1_planning_item_2.md)</sub>
    - [x] Add tests for `Secret` type and update existing token tests <sub>[ctx_rec_5](https://github.com/milyin/zbobr/blob/reports/reports/task_238/checklist_main_1_planning_item_3.md)</sub>
<!-- stage -->
- skynet:main:1:**working** `claude` `claude-sonnet-4.6` `2026-03-29 01:48:32 +0100`
  - ✅ Secret type implemented: zbobr-api/src/secret.rs with Value/Env variants, all 4 token fields migrated, callsites updated, 9 tests added, all tests pass <sub>[ctx_rec_7](https://github.com/milyin/zbobr/blob/reports/reports/task_238/report_main_1_working_report_success.md)</sub>
<!-- stage -->
- skynet:main:1:**reviewing** `claude` `claude-haiku-4.5` `2026-03-29 03:03:14 +0200`
  - ✅ Secret type implementation complete and correct. All 4 token fields migrated, tests pass (153 total), and implementation pattern is consistent with codebase analog (StageTransition). <sub>[ctx_rec_8](https://github.com/milyin/zbobr/blob/reports/reports/task_238/report_main_1_reviewing_report_success.md)</sub>
<!-- stage -->
- skynet:main:1:**testing** `claude` `claude-haiku-4.5` `2026-03-29 03:08:34 +0200`
  - ✅ Secret type implementation complete and fully tested: 117 tests pass, 4 token fields migrated, backward compatibility removed, formatting fixed <sub>[ctx_rec_9](https://github.com/milyin/zbobr/blob/reports/reports/task_238/report_main_1_testing_report_success.md)</sub>
- user:**milyin** Additional requirement. 
The secret type should support two access methods:
`fn resolve(&mut self) -> Result<&str, ...>` - tries to fetch the secret value, returns error if failed. Called on the validation stage
`impl AsRef<str>` - panics if `resolve` was never called or if it failed. Otherwise returns the ref to stored secret string.
Important: even if `Secret` is actually initialized from static string and secret string is technically available, `resolve` still have to be called before accessing `deref` - otherwise do the panic on `deref`.
I.e. `Secret` can be serialized and deserialized without resolving, but to access secret value the resolving is obligatory.
This means that the secret type becomes structure with private fields, public enum form is not needed anymore.
 `2026-03-29 10:49:29 +0000` <sub>[link](https://github.com/milyin/zbobr/issues/238#issuecomment-4149895545)</sub>
<!-- stage -->
- skynet:main:1:**preparing** `copilot` `gpt-5-mini` `2026-03-29 13:37:14 +0200`
<!-- stage -->
- skynet:main:1:**planning** `claude` `claude-sonnet-4.6` `2026-03-29 13:38:46 +0200`
  - 💬 Plan: refactor Secret into struct with private fields, resolve(&mut self) caches value, AsRef<str> panics if unresolved <sub>[ctx_rec_10](https://github.com/milyin/zbobr/blob/reports/reports/task_238/report_main_1_planning_report_intermediate_1.md)</sub>
- user:**milyin** Implement the plan with these corrections: 
- clone should not clear resolved state, cloned resolved secret is resolved
- add "is_resolved()" method
- move secret struct from API to utilities crate (I'm sure it still exist) 
- no need to check "is_resolved" in work code, it's added just for completeness, early panic is better for current project `2026-03-29 13:23:12 +0000` <sub>[link](https://github.com/milyin/zbobr/issues/238#issuecomment-4150155146)</sub>
<!-- stage -->
- skynet:main:1:**planning** `claude` `claude-sonnet-4.6` `2026-03-29 15:25:17 +0200`
<!-- stage -->
- skynet:main:1:**planning** `claude` `claude-sonnet-4.6` `2026-03-31 13:15:29 +0200`
  - ✅ Plan ready: 3 items — Secret struct in zbobr-utility, re-export in zbobr-api, callsite migration <sub>[ctx_rec_14](https://github.com/milyin/zbobr/blob/reports/reports/task_238/report_main_1_planning_report_success_1.md)</sub>
    - [ ] Implement Secret struct in zbobr-utility <sub>[ctx_rec_11](https://github.com/milyin/zbobr/blob/reports/reports/task_238/checklist_main_1_planning_item_7.md)</sub>
    - [ ] Update zbobr-api: remove secret.rs, re-export Secret from zbobr-utility <sub>[ctx_rec_12](https://github.com/milyin/zbobr/blob/reports/reports/task_238/checklist_main_1_planning_item_8.md)</sub>
    - [ ] Update validate() signatures, construction callsites, and token access callsites <sub>[ctx_rec_13](https://github.com/milyin/zbobr/blob/reports/reports/task_238/checklist_main_1_planning_item_9.md)</sub>
<!-- stage -->
- skynet:main:1:**working** `claude` `claude-sonnet-4.6` `2026-03-31 13:21:59 +0200`
