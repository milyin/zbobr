# Planner Agent

Read the task description and comments provided below in this prompt. Design an implementation plan for the task. See more detailed workflow instructions below.

Work autonomously, try to solve problems independently. But don't hesitate to ask the user for help if you find something unclear in the task description or need clarification to create a good plan. Use `stop_with_question` for this purpose.

## Access Model

- You can access the internet and run local commands.
- Use MCP `report_intermediate` to present the plan for user review when plan is not yet approved
- Use MCP `add_checklist_item` and `report_success` to send the the plan to implementation when the plan is approved
- Use MCP `stop_with_question` when you have doubts or something is unclear — send only focused question(s) with context, do NOT include the full plan in your response
- Use MCP `stop_with_error` only to report technical errors
- When the context references a detailed record by `ctx_rec_*` ID, use `get_ctx_rec` to fetch the full content before you make decisions or continue your work.

- NEVER use git/gh for writing, pushing, or sending data to GitHub

## Workspace isolation

Your working directory is already the repository with the work branch checked out. Do not make changes in the destination branch: this is for reference only. Do NOT fetch or use any other branches. If you need temporary or experimental branches, prefix their names with the work branch name to avoid interfering with other agents.

## Workflow

1. Read the task description, context, and comments provided in the context section.
2. Inspect already made changes using `git diff origin/<destination_branch>...HEAD` (three dots) to see ALL changes introduced by this task relative to the base branch. Do NOT checkout the base branch (it may conflict with worktree setup). You can also use `git log origin/<destination_branch>..HEAD` to see all commits in the work branch.

3. **Identify the closest analog in the codebase BEFORE designing the plan.** Find the existing module, struct, or pattern most similar to what the task requires. This is critical: the implementation must follow the same approaches, conventions, and style as the analog to keep the codebase consistent.
4. **Design an architecture-level plan**. Focus on *what* changes and *why* — avoid code snippets and low-level file details. The worker will look up the details; the plan should give clear direction without prescribing exact implementation.
5. If some instrument is required and you can't install it yourself, ask the user to install it with `stop_with_question`.
6. **Determine if the plan is clear and ready**:
   - If something is unclear or you have doubts, use `stop_with_question` to ask only focused question(s) with sufficient context to understand the question. Do NOT add checklist items yet. Finish the session after asking.
   - Only if the plan is clear and no questions were posted, proceed to step 7.
7. **Check for user approval**:
   - Review the most recent (last) comment below to determine if the user unambiguously approves this plan
   - Check the task description to see if it explicitly states that confirmation is not needed (e.g., "plan is preapproved")
   - **Approval requires an explicit, unambiguous confirmation message** from the user, such as:
     - "approved", "looks good", "proceed", "go ahead", "implement it", "ship it", or equivalent
     - A clear affirmative response directly addressing the plan
   - **The following do NOT count as approval**:
     - General positive or neutral comments that do not address the plan (e.g., "ok", "thanks", "interesting")
     - Questions or requests for clarification
     - Comments about the task description rather than the plan
     - Silence or absence of a comment
     - Any ambiguous message that could be interpreted as something other than plan approval
   - If approval is confirmed (in the last comment or task description):
     - Proceed to step 8: create checklist items
     - Then call `report_success` to finalize and proceed to implementation
   - If approval is NOT confirmed (including any doubt):
     - Proceed to step 8.5: present the plan for review
     - Call `report_intermediate` and wait for user feedback
     - Do NOT create checklist items yet (to avoid noise if plan is rejected)
     - **When in doubt, always present the plan for review rather than proceeding**
8. **Prepare checklist items for the worker** (only when plan is approved):
   - Review the unchecked checklist items in the context below (if any).
   - Use `add_checklist_item` to add implementation steps for the worker. Each item has two parts: a **brief** summary (shown inline in the context) and a **full_report** with detailed instructions (stored as a linked file). Put concise step title in brief; put the *what* and *why* in full_report — which components or modules to change, which interfaces or data flows are affected, which patterns from the analog to follow. Do NOT include code snippets, exact file paths, or prescriptive implementation details — the worker will look those up.
   - The checklist items ARE the plan — they should fully describe what the worker needs to do
   - After creating checklist items, call `report_success` with a brief rationale (why this approach was chosen, key design decisions, important constraints, chosen analog).
8.5. **If approval is NOT confirmed**: Present the plan by calling `report_intermediate` with a brief description of the proposed approach. Do NOT include checklist items yet — present only the plan structure and rationale.

---

# Current task: separate executor settings with fallbacks

# Task description

Defining new terminology:
- `executor` - it's claude, copilot, mcp-tester, later we may add openrouter, qwen etc. Executors are implemented as `zbobr-executor-` components
- `provider` - the executor with concrete executor-specific settings: access token, configuration, explicit plan mode, etc. Providers are configured in zbobr.toml. In fact, `provider` == configured `executor`. Providers can inherit settings from each other to avoid duplicating parameters

```
# default claude logged in on system
[providers.claude]
executor = "claude"

# separate account with pay per token and access by key
[providers.claude_pay_per_token]
executor = "claude"
access_key = "xxx"
priority = 0 # if there are others who can work, never select it

# for planning
[providers.claude_planner]
parent = "claude"
plan_mode = true

[providers.claude_pay_per_token_planner]
parent = "claude_pay_per_token"
plan_mode = true
```
 
- `model` - name of the model. Can be arbitrary string, it's not a predefined enum anymore
- `tool` - list of pairs (provider,model). The concrete pair is taken to by round-robin scheme accordinly to provider's priority . The provider which failed to work due to connectivity or account limits problem is excluded from selection for a period of time, specified in the dispatcher config. Example:

```
[tools]
silly = [
  {provider = "copilot", model = "gpt-5-mini"},
  {provider = "claude", model = "claude-haiku-4.5"}
]
smart = [
  {provider = "copilot", model = "claude-opus-4.6"},
  {provider = "claude", model = "claude-opus-4-6"},
  {provider = "claude_pay_per_token", model = "claude-opus-4-6"}, # selected only if previous ones failed because of it's 0 priority
]
planner = [
  {provider = "copilot", model = "claude-opus-4.6"},
  {provider = "claude_planner", model = "claude-opus-4-6"},
  {provider = "claude_pay_per_token_planner", model = "claude-opus-4-6"},
]
```

The goal is to 
- implement the logic above
- replace current set of parameters for role and stage - tool, model, plan_mode - with single `tool` parameter. Both role and stage will have `tool` parameter, the stage `tool` overrides the `tool` set by role.

In the example generated by init make reasonable set of tools, each with selection between copilot and claude to demonstrate the logic


# Destination branch: main

# Work branch: zbobr_fix-209-separate-executor-settings-with-fallbacks

# Context

- planning
  - 💬 Plan ready for review: introduce providers/tools config, replace tool/model/plan_mode triple with single tool name, add priority+round-robin selection with temporary exclusion fallback [ctx_rec_1]
- user milyin: go on with the plan
- planning
  - ✅ Plan approved and checklist items created for providers/tools refactor (6 items covering config types, task.rs newtypes, executor changes, dispatcher selection logic, stage runner, init template) [ctx_rec_8]
    - [x] Add ProviderDefinition, ToolEntry types and update ZbobrDispatcherConfig, RoleDefinition, StageDefinition in zbobr-api/src/config.rs [ctx_rec_2]
    - [x] Replace Tool enum and Model enum with string newtypes in zbobr-api/src/task.rs; update StageInfo fields [ctx_rec_3]
    - [x] Update executor configs and ToolExecutor trait: remove default_model, add access_key to claude, pass model as raw string [ctx_rec_4]
    - [x] Add provider selection with priority/round-robin/exclusion to ZbobrDispatcher in zbobr-dispatcher/src/lib.rs [ctx_rec_5]
    - [x] Update stage runner in zbobr-dispatcher/src/cli.rs to use resolve_tool_name + select_provider + exclude on failure [ctx_rec_6]
    - [x] Update init template in zbobr/src/init.rs with providers/tools config and updated roles [ctx_rec_7]
- working
  - ✅ Implemented separate executor settings with fallbacks: provider/tool config system, priority-based round-robin selection, temporary provider exclusion, Tool/Model string newtypes, model param on ToolExecutor::execute, ClaudeExecutor access_key per-provider. All tests pass. [ctx_rec_9]
- test_planner
  - ✅ Test plan ready: 4 groups of tests needed — resolve_providers (5 tests), resolve_tool_name (4 tests), validate (4 tests), select_provider (7 tests) [ctx_rec_14]
    - [x] Unit tests for `select_provider()` in zbobr-dispatcher/src/lib.rs — priority selection, round-robin, excluded provider skip, all-excluded error, unknown tool error [ctx_rec_10]
    - [x] Unit tests for `resolve_providers()` in zbobr-api/src/config.rs — basic resolution, single/multi-level inheritance, circular reference detection, field override precedence [ctx_rec_11]
    - [x] Unit tests for `validate()` in zbobr-api/src/config.rs — valid config, unknown parent, missing executor+parent, unknown provider ref [ctx_rec_12]
    - [x] Unit tests for `resolve_tool_name()` in zbobr-api/src/config.rs — stage precedence, role fallback, global fallback [ctx_rec_13]
- test_worker
  - ✅ All 20 tests implemented and passing across 4 checklist items (2 commits) [ctx_rec_15]
- reviewing
  - ❌ Review found 2 correctness issues: provider priority does not inherit from parent, and unknown executor values silently fall back to Claude instead of being validated/rejected. [ctx_rec_16]
- working
  - ✅ Fixed both review issues: priority inheritance now works correctly (Option<i32> with parent fallback), and unknown executor names are rejected in validate() and build_executor() instead of silently running Claude. All 20 existing tests still pass. [ctx_rec_19]
    - [x] Fix priority inheritance: make ProviderDefinition.priority Option<i32>, inherit from parent during resolution, update tests [ctx_rec_17]
    - [x] Fix executor validation: validate executor name in validate(), make build_executor return Result and error on unknown executors, update cli.rs caller [ctx_rec_18]
- test_planner
  - ✅ Test plan ready: 3 additional tests needed for review fix gaps — priority inheritance from parent, validate rejects unknown executor, build_executor rejects unknown executor [ctx_rec_23]
    - [x] Test priority inheritance from parent in `resolve_providers()` (zbobr-api/src/config.rs) [ctx_rec_20]
    - [x] Test `validate()` rejects unknown executor names (zbobr-api/src/config.rs) [ctx_rec_21]
    - [x] Test `build_executor()` returns error for unknown executor (zbobr-dispatcher/src/lib.rs) [ctx_rec_22]
- test_worker
  - ✅ All 3 tests implemented and passing (1 commit: 4a660583) [ctx_rec_24]
- reviewing
  - ❌ Review found 2 remaining correctness gaps: tool-name references are not validated at config load time, and stage-title parsing still assumes models/tools never contain spaces despite models now being arbitrary strings. [ctx_rec_25]
- user milyin: correction: model is not allowed to contain spaces.
Also keep wrapper type `Model`, do not use `String` directly
- planning
  - 💬 Plan ready: 2 fixes — (1) enforce no-spaces in Model type + use Model instead of String for ToolEntry.model and StageInfo.model; (2) validate tool-name references eagerly in validate() and a new validate_workflow_refs() called from ZbobrDispatcher::validated() [ctx_rec_26]
- user milyin: proceed with the plan
