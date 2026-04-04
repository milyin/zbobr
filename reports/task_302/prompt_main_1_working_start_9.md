# Worker Agent

Implement the task accordingly to the final plan in the context. Notice that there can be multiple plan versions in the history, work on the last one. If the plan is accompanied by checklist items, process them one by one, skip the checked ones. If there are no checklst items, analyze the pan and create checklist items for the implementation steps yourself.

- Use `check_checklist_item` to mark item as done when you complete the subtask in it.
- Use `add_checklist_item` to add new item when you discover new job to do or user made additional request in comments.
- When the context references a detailed record by `ctx_rec_*` ID, use `get_ctx_rec` to fetch the full content before you make decisions or continue your work.


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
5. If you sense your context window is getting close to its limit, finish your current item to a buildable state, commit your work, mark completed items as done, call `report_intermediate` with a summary of what you accomplished and what remains and finish the session.
6. If you need human clarification or intervention, call `stop_with_question`. If the plan is unclear or requires adjustment, call `report_failure`. In case of technical errors use `stop_with_error`.
7. If some instrument is required and you can't install it yourself, ask the user to install it with `stop_with_question`.
8. When your current session's work is done, decide how to finish:
    - If **all checklist items are completed** (the full plan is done), call `report_success` to report final success.
    - If **some items remain unchecked** (more work is needed in future sessions), call `report_intermediate` to report what you accomplished so far.

## Coding Guidelines

- **Prefer deriving values from types and constants** rather than using hardcoded string literals. If a value can be computed from an existing type, enum variant, or constant, do it. Avoid duplicating the value as literals or constants.

---

# Current task: allow configuration sharing

# Task description

Multiple zbobr instances should be able to share common pipeline and template logis by only making patches with project specific settings.
To do it multiple config loading should be supported. Make it with following logic:
- allow to pass multiple `--config` parameters (add `-c` shortcut to make it easier)
- when one or more `--config` passed, the default `zbobr.toml` is ignored
- apply configs in order of appearance, next one overrides previous one
- named parameters override parameters with the same name
- list-type parameter appears treated as whole values, they fully replaces previous list
- do not do any changes to current config structure, it will be adapted in separate task

# Destination branch: main

# Work branch: zbobr_fix-302-allow-configuration-sharing

# Context

- planning
  - 💬 Plan: Add multi-config support. (1) Change ConfigFileArg to Vec<PathBuf> with -c [ctx_rec_1]
- user milyin: proceed with the plan
- planning
  - ✅ Plan approved and checklist created for multi-config support implementation. [ctx_rec_7]
    - [x] Change ConfigFileArg to accept multiple config paths with -c shortcut [ctx_rec_2]
    - [x] Add merge_toml method to Toml structs via config_struct macro [ctx_rec_3]
    - [x] Update resolve_config_location for multiple paths [ctx_rec_4]
    - [x] Merge multiple configs in main.rs loading flow [ctx_rec_5]
    - [x] Update all callers of ConfigFileArg and resolve_config_location [ctx_rec_6]
- user milyin: The task was mostly implemented.
Now it's assumed that Vec is always under Option and that empty/missing vector should not be distinguished.
Verify, test
- planning
  - 💬 Verification complete. Vec-under-Option assumption is correct. Proposing test ad [ctx_rec_8]
- user milyin: proceed with plan
- planning
  - ✅ Comprehensive understanding of TOML config merging testing patterns found and do [ctx_rec_9]
    - [x] Add TOML deserialization round-trip tests for Option<Vec> fields [ctx_rec_10]
    - [x] Add tools map merge test (IndexMap<String, Vec<ToolEntry>> key-wise merge) [ctx_rec_11]
    - [x] Add end-to-end multi-config merge test from TOML strings [ctx_rec_12]
  - ✅ Plan approved and checklist created for 3 test additions to lock down multi-conf [ctx_rec_13]
- working
  - ✅ All 3 test checklist items implemented: Option<Vec<T>> serde round-trips (6 test [ctx_rec_14]
- reviewing
  - ❌ Found a blocking regression unrelated to multi-config merging: roles with omitte [ctx_rec_15]
