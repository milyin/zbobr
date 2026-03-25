# Reviewer Agent

Review the implementation changes and ensure they meet coding standards and task requirements.

## Access Model

    You have read-only access to the task plan and access to the repository for inspection:
    - The task description, work plan, worker's report, comments, and checklist are provided below in this prompt. Use `get_history` to read the full discussion history if needed for more context.
    - Your current working directory is already the repository with the work branch checked out — examine changes directly
    - Use `stop_with_error` only to report technical errors

## Workflow

1. Read the task description, work plan, worker's report, comments, and checklist provided below in this prompt. Note if the analog solution in the existing code is referenced in the plan.
2. **Inspect all changes made in this task**: Use `git diff origin/<destination_branch>...HEAD` (three dots) to see ALL changes introduced by this task relative to the base branch. Do NOT checkout the base branch (it may conflict with worktree setup). You can also use `git log origin/<destination_branch>..HEAD` to see all commits in this branch.
3. **Verify the analog choice and pattern consistency**: Check that the planner chose an appropriate analog for the new functionality. Then verify that the implementation consistently follows the same patterns, conventions, coding style, and architectural approaches as the analog. Flag any deviations — new code should look like it was written by the same author as the existing analogous code. If the analog was poorly chosen, note this as a review finding.
4. **Review code quality and correctness**: Examine the implementation for correctness, code style, design patterns, and adherence to the plan. **Do not run any tests yourself; testing is handled in a separate Testing stage.**
5. Verify that all changes are related to the task and are necessary for the implementation. Flag any extraneous changes that do not directly contribute to the task requirements or plan.
6. Prepare a detailed review report describing any issues found, suggested fixes, and overall assessment. Include your assessment of analog consistency.
7. Call `report_success` if the implementation is correct and complete, or `report_failure` if issues were found. Pass the review report as a parameter to these tools.

---

# Current task: `context` structure instead of comments feed

# Task description

Storing work result in the feed of comments makes it hard to analyze the task, observe it, and control the context size.

Also splitting the context between checkboxes and comments makes it hard to follow the logic of task resolution.

The solution:

Task:

- create structure `TaskContext`, use it as `Task` field
- the `TaskContext` contains 
   - `Vec<StageContext>`
- the `StageContext` contains 
  - `StageInfo` - structure with pipeline, stage, tool, model, link to prompt, timestamp
  - `Vec<ContextRecord>`
  - optional user's comment
- the `ContextRecord` contains
  - unique numeric id, generated on dispatcher side as max records id + 1
  - enum `ContextRecordType`: checkbox(bool), success ✅ , failure ❌, comment, question ?
  - brief description
  - optional link to long description / report
- remove the checklist component from the Task, remove keeping different checkists for pipeline runs, remove filtering checlist by only checked items, etc.
- store md-formatted `TaskContext` in the task description ans parse it from there. In case of parsing problems immediately report error, do not try to make assumptions. 

Prompt templating:

Add placeholder {context} which inserts md formatted `TaskContext` to the output.

Context MD output:
- add parameter `prompt: bool` to md-generating function. When set, do not output links to prompts. More restrictions can be added later. Use this parameter for prompt template
- format record ids as <subr>[ctx_rec_{id}]</sub> in the end of each record
- pass user's comments to MD writer. The md output is interspersed with user's comments placed accordingly to timeline. When parsing this interspersed md, comments are ignored. I.e. the only authoritative source of comments is the real comments feed, the comments in the context are just to provide them to agent in the correct places and to show the discussion history in the task decription.

MCP:
- remove `GetHistory` and `GetChecklist` mcp methods. The whole history and checlist is available as context now
- remove `GetFullReport`. The links to reports are just normal links (http for gihtub backend, filesystem paths for fs backend)
- remove `DeleteChecklistItem`
- add  `DeleteCtxRec` which accepts either numeric id or string `ctx_rec_id`
- method `AddChecklistItem` should accept optional long description which is stored to file, in the sane way as success / falure reports

Prompts:
- update accordingly to changes in templating and mcp 
- allow to send multiple success / fauilure reports, tell about this possibility
- the reviewer and tester component should not add checkboxes (disable this mcp for them). Instead they can send multiple success / failure reports

# Destination branch: main

# Work branch: zbobr_fix-163-context-structure

# Last report

Step 1 complete: Added TaskContext data structures to zbobr-api/src/task.rs. New types: ContextRecordType enum (Checkbox(bool), Success, Failure, Comment, Question), ContextRecord (id, record_type, brief, report_link), StageInfo (pipeline, stage, tool, model, prompt_link, timestamp), StageContext (info, records, user_comment), TaskContext (stages Vec) with methods next_id(), find_record(), find_record_mut(), delete_record(), Default impl. Added Task.context field alongside existing checklist. Exported all new types from lib.rs. 6 new unit tests pass, all 23 total tests pass. Committed as 9fcc9c8.

[report_main_1_working_success.md](https://github.com/milyin/zbobr/blob/reports/reports/task_163/report_main_1_working_success.md)

# Last request

Plan is not fully correct. The comments should not be posted at all. Stages results are stored in context only, no duplication

# Unchecked checklist items

- [ ] [id: step-2-context-format] Create zbobr-api/src/context_format.rs replacing checklist_format.rs. Implement serialize_context(ctx, comments, for_prompt) generating MD with stage headers (<!-- Stage: pipeline #run_id stage [timestamp] -->), record lines with type prefixes and <sub>[ctx_rec_{id}]</sub> tags, interspersed user comments by timestamp. Implement parse_context(text) -> Result<TaskContext> that parses this MD format back, ignoring user comments, returning Err on parse failures. Delete checklist_format.rs, update lib.rs module declaration.
- [ ] [id: step-3-separator] Update zbobr-task-backend-github/src/separator.rs: replace CHECKLIST_SEPARATOR with CONTEXT_SEPARATOR ("---CONTEXT---"). Change parse_description_full to return Result<(String, HashMap, Option<String>, TaskContext)> using context_format::parse_context. Update serialize_description_full to accept &TaskContext. Update merge_concurrent_description_updates for context. Fix all tests.
- [ ] [id: step-4-backends] Update both backends: In zbobr-task-backend-fs/src/fs.rs replace TaskFile.checklist with context: TaskContext, update to_task/from_task. In zbobr-task-backend-github/src/github.rs update issue_to_task to handle Result from parse_description_full, update modify_task serialization. Replace all task.checklist references with task.context across both backends.
- [ ] [id: step-5-mcp-tools] Update MCP tools: In config_tools.rs remove GetHistory, GetChecklist, GetFullReport, DeleteChecklistItem variants; add DeleteCtxRec. Update as_str/FromStr/ALL_TOOLS/ALL_TOOL_NAMES. In common.rs add DeleteCtxRecParam {id: String}, add long_description: Option<String> to AddChecklistItemParam, remove DeleteChecklistItemParam and GetFullReportParam. In unified.rs remove get_history/get_checklist/get_full_report/delete_checklist_item tool methods, add delete_ctx_rec. In traits.rs remove corresponding _impl methods, add delete_ctx_rec_impl (parse id stripping ctx_rec_ prefix), update add_checklist_item_impl for long_description (store as file, set report_link), update report_impl to also add ContextRecord to TaskContext, update check_checklist_item_impl for ContextRecord.
- [ ] [id: step-6-role-session] Update RoleSession in zbobr-dispatcher/src/task.rs: Remove get_checklist, add_checklist_item, check_checklist_item, delete_checklist_item, checklist_scope_prefix, strip_checklist_scope. Add context methods: add_context_record(record_type, brief, report_link) -> Result<u64>, ensure_current_stage(task) -> &mut StageContext, check_context_record(id) -> Result<bool>, delete_context_record(id) -> Result<bool>. No more pipeline scoping — context records have global numeric IDs.
- [ ] [id: step-7-prompts] Update prompt templating in zbobr-dispatcher/src/prompts.rs: Replace VAR_CHECKLIST with VAR_CONTEXT = "context". In build_template_variables remove checklist filtering/scoping, add {context} using context_format::serialize_context(&task.context, comments, true). Remove filter_and_strip_scope import. Update all tests.
- [ ] [id: step-8-stage-creation] In zbobr-dispatcher/src/cli.rs CliStageRunner::run(): after allocating pipeline_run_id and before starting MCP server, create a StageContext with StageInfo (pipeline, stage, tool, model, timestamp) and add it to task.context.stages via modify_task. After prompt is built, update the stage's prompt_link with the stored prompt filename.
- [ ] [id: step-9-cleanup-tests] Cleanup and fix all compilation errors: update all dummy_task/test helpers to use context: TaskContext::default() instead of checklist: vec![]. Update integration tests in zbobr-dispatcher/tests/. Remove ChecklistItem struct if fully unused. Run cargo build and cargo test to verify everything compiles and passes.