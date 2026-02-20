use std::sync::Arc;

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde_json::Value;

use crate::{
    Zbobr,
    task::{ChecklistItem, Parameter, Role, TaskSession},
};

// Instruction shared across all role prompts explaining branch isolation rules.
fn branch_isolation_instruction() -> String {
    use planner_tools::{GET_PARAM_DESTINATION_BRANCH, GET_PARAM_WORK_BRANCH, PULL_WORK};
    format!(
        "Workspace branch isolation. Always start work woith the {PULL_WORK}.
        In the project returned by it use ONLY the destination 
        and work branches with names provided by the MCP tools `{GET_PARAM_DESTINATION_BRANCH}`, 
        `{GET_PARAM_WORK_BRANCH}`. Do not make changes in the destination branch: this is
        for reference only. Do NOT fetch or use any other branches. Do NOT look at branches 
        other than the work and destination branches. If you need temporary or experimental branches, 
        prefix their names with the work branch name to avoid interfering with other agents.",
    )
}

/// Get the current hostname, or "unknown" if it cannot be determined.
fn get_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
}

// -- Parameter types --

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct DescriptionParam {
    #[schemars(description = "The task description/plan text")]
    pub description: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct MessageParam {
    #[schemars(description = "The message to post")]
    pub message: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct RepoParam {
    #[schemars(description = "Target repository (full git URL, local path, or owner/repo)")]
    pub repo: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct PathParam {
    #[schemars(description = "Local filesystem path to repository")]
    pub path: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct ShortNameParam {
    #[schemars(description = "Short name for the branch (e.g. 'implementation', 'fix-typo')")]
    pub short_name: String,
}

// -- Checklist parameter types --

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct InsertChecklistItemParam {
    #[schemars(description = "Unique identifier for the new checklist item")]
    pub id: String,
    #[schemars(description = "Checklist item text")]
    pub text: String,
    #[schemars(description = "Optional ID of the item to insert after (if omitted, adds to end)")]
    pub after_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct UpdateChecklistItemParam {
    #[schemars(description = "ID of the checklist item to update")]
    pub id: String,
    #[schemars(description = "New text for the checklist item")]
    pub text: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct CheckChecklistItemParam {
    #[schemars(description = "ID of the checklist item to check/uncheck")]
    pub id: String,
    #[schemars(description = "New checkbox state (true = checked, false = unchecked)")]
    pub checked: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct DeleteChecklistItemParam {
    #[schemars(description = "ID of the checklist item to delete")]
    pub id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct SetDestinationRepositoryParam {
    #[schemars(
        description = "Destination repository (full git URL, local path, or owner/repo format) (or null to unset)"
    )]
    pub value: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct SetDestinationBranchParam {
    #[schemars(
        description = "Work branch postfix (the final segment after prefix/task_id) (or null to unset)"
    )]
    pub value: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct SetWorkBranchParam {
    #[schemars(description = "Work branch name (or null to unset)")]
    pub value: Option<String>,
}

macro_rules! mcp_tools {
    ($mod_name:ident, $($name:ident = $val:expr),* $(,)?) => {
        pub mod $mod_name {
            $(pub const $name: &str = $val;)*
            pub const ALL_TOOLS: &[&str] = &[$($val),*];
        }
    }
}

mcp_tools! {
    preparator_tools,
    GET_DESCRIPTION = "get_description",
    GET_DISCUSSION = "get_discussion",
    REPORT_ERROR = "report_error",
    SET_PARAM_DESTINATION_REPOSITORY = "set_param_destination_repository",
    SET_PARAM_DESTINATION_BRANCH = "set_param_destination_branch",
    SET_PARAM_WORK_BRANCH_POSTFIX = "set_param_work_branch_postfix",
    GET_PARAM_DESTINATION_REPOSITORY = "get_param_destination_repository",
    GET_PARAM_DESTINATION_BRANCH = "get_param_destination_branch",
    GET_PARAM_WORK_BRANCH = "get_param_work_branch",
    REPORT_RESULTS = "report_results",
}

mcp_tools! {
    planner_tools,
    GET_DESCRIPTION = "get_description",
    GET_DISCUSSION = "get_discussion",
    GET_PLAN = "get_plan",
    POST_PLAN = "post_plan",
    REPORT_ERROR = "report_error",
    PULL_WORK = "pull_work",
    GET_PARAM_DESTINATION_BRANCH = "get_param_destination_branch",
    GET_PARAM_WORK_BRANCH = "get_param_work_branch",
    REPORT_RESULTS = "report_results",
}

mcp_tools! {
    worker_tools,
    GET_DESCRIPTION = "get_description",
    GET_DISCUSSION = "get_discussion",
    GET_PLAN = "get_plan",
    REPORT_ERROR = "report_error",
    ASK_USER = "ask_user",
    ASK_PLANNER = "ask_planner",
    PULL_WORK = "pull_work",
    PUSH_WORK = "push_work",
    GET_CHECKLIST = "get_checklist",
    INSERT_CHECKLIST_ITEM = "insert_checklist_item",
    UPDATE_CHECKLIST_ITEM = "update_checklist_item",
    CHECK_CHECKLIST_ITEM = "check_checklist_item",
    DELETE_CHECKLIST_ITEM = "delete_checklist_item",
    GET_PARAM_DESTINATION_BRANCH = "get_param_destination_branch",
    GET_PARAM_WORK_BRANCH = "get_param_work_branch",
    REPORT_RESULTS = "report_results",
}

mcp_tools! {
    reviewer_tools,
    GET_DESCRIPTION = "get_description",
    GET_PLAN = "get_plan",
    REPORT_ERROR = "report_error",
    PULL_WORK = "pull_work",
    INSERT_CHECKLIST_ITEM = "insert_checklist_item",
    GET_PARAM_DESTINATION_BRANCH = "get_param_destination_branch",
    GET_PARAM_WORK_BRANCH = "get_param_work_branch",
    REPORT_RESULTS = "report_results",
}

mcp_tools! {
    merger_tools,
    GET_DESCRIPTION = "get_description",
    GET_DISCUSSION = "get_discussion",
    REPORT_ERROR = "report_error",
    ASK_USER = "ask_user",
    PULL_WORK = "pull_work",
    PUSH_WORK = "push_work",
    GET_PARAM_DESTINATION_BRANCH = "get_param_destination_branch",
    GET_PARAM_WORK_BRANCH = "get_param_work_branch",
    REPORT_RESULTS = "report_results",
}

/// Generate hardcoded preparator instructions using tool name constants.
pub fn preparator_instructions() -> String {
    use preparator_tools::{
        GET_DESCRIPTION, GET_DISCUSSION, GET_PARAM_DESTINATION_BRANCH,
        GET_PARAM_DESTINATION_REPOSITORY, GET_PARAM_WORK_BRANCH, REPORT_ERROR, REPORT_RESULTS,
        SET_PARAM_DESTINATION_BRANCH, SET_PARAM_DESTINATION_REPOSITORY,
        SET_PARAM_WORK_BRANCH_POSTFIX,
    };
    use worker_tools::ASK_USER;
    format!(
        r#"# Preparator Agent

Read the task description and set the required parameters for the implementation.

## Access Model

    You can access the internet and run local commands. Your restrictions:
    - Use MCP `{REPORT_ERROR}` only to report technical errors
    - Use `{ASK_USER}` to request the user's explanations related to the task
    - For reading GitHub data: use `git` and `gh` CLI only when no MCP tool provides the needed information
    - NEVER use git/gh for writing, pushing, or sending data to GitHub

## Workflow

1. Call `{GET_DESCRIPTION}` to read the user task description
2. Call `{GET_DISCUSSION}` for context and prior comments
3. **Set task parameters** that will guide the implementation:
    - Call `{SET_PARAM_DESTINATION_REPOSITORY}` with the target repository (full git URL, local path, or owner/repo format)
    - Call `{SET_PARAM_DESTINATION_BRANCH}` (e.g., "main", "develop")
    - Call `{SET_PARAM_WORK_BRANCH_POSTFIX}` with the work branch postfix (e.g., "implement-feature") — the full work branch will be formed from prefix, task id and this postfix
    - Use `{GET_PARAM_DESTINATION_REPOSITORY}`, `{GET_PARAM_DESTINATION_BRANCH}`, `{GET_PARAM_WORK_BRANCH}` to read current values
4. Call `{REPORT_RESULTS}` to provide a brief and concise report of your work and finish the session. This report takes part in the context for further agent calls, so it MUST be compact.
5. When finished, the task will move to the planning stage.
"#,
    )
}

/// Generate hardcoded planner instructions using tool name constants.
pub fn planner_instructions() -> String {
    use planner_tools::{
        GET_DESCRIPTION, GET_DISCUSSION, GET_PARAM_DESTINATION_BRANCH, GET_PARAM_WORK_BRANCH,
        GET_PLAN, POST_PLAN, PULL_WORK, REPORT_ERROR, REPORT_RESULTS,
    };
    use worker_tools::ASK_USER;
    let branch_isolation = branch_isolation_instruction();
    format!(
        r#"# Planner Agent

Investigate a task and create an implementation plan with actionable steps.

## Access Model

    You can access the internet and run local commands. Your restrictions:
    - Do NOT run git clone/pull/fetch — use `{PULL_WORK}` instead
    - Use MCP `{POST_PLAN}` to post the implementation plan
    - Use MCP `{REPORT_ERROR}` only to report technical errors; use `{ASK_USER}` to request the user's explanations related to the task
    - For reading GitHub data: use `git` and `gh` CLI only when no MCP tool provides the needed information
    - NEVER use git/gh for writing, pushing, or sending data to GitHub

## Workspace isolation

    {branch_isolation}

Work autonomously. Do not ask the user for anything.

## Workflow

1. Call `{GET_DESCRIPTION}` to read the user task description
2. Call `{GET_PLAN}` to read an existing plan if there is one
3. Call `{GET_DISCUSSION}` for context and prior comments and questions to existing plan
4. **Task parameters** have already been set by the preparation stage:
    - Use `{GET_PARAM_DESTINATION_BRANCH}`, `{GET_PARAM_WORK_BRANCH}` to read branch names if needed.
5. Pull the destination repository using `{PULL_WORK}` to investigate the codebase, understand the context, and design the plan. This also ensures the repo is cached for the worker later.
6. Explore the codebase, identify and document the files, crates, modules, and keywords relevant to the task. These help define the scope and guide the worker:
   - List specific files that need to be modified or created
   - Identify crates/modules that contain related functionality
   - Include keywords/concepts the worker should focus on (e.g., "async/await", "error handling", "API compatibility")
   - This context narrows the worker's scope and prevents unnecessary exploration
7. Design a solution. 
8. Post a solution in the form of a text plan with `{POST_PLAN}`. Use planning mode if available.
9. Call `{REPORT_RESULTS}` to provide a brief and concise report of your work and finish the session. This report takes part in the context for further agent calls, so it MUST be compact.
"#,
    )
}

/// Generate hardcoded worker instructions using tool name constants.
pub fn worker_instructions() -> String {
    use worker_tools::{
        ASK_PLANNER, ASK_USER, CHECK_CHECKLIST_ITEM, DELETE_CHECKLIST_ITEM, GET_CHECKLIST,
        GET_DESCRIPTION, GET_DISCUSSION, GET_PARAM_DESTINATION_BRANCH, GET_PARAM_WORK_BRANCH,
        GET_PLAN, INSERT_CHECKLIST_ITEM, PULL_WORK, PUSH_WORK, REPORT_ERROR, REPORT_RESULTS,
        UPDATE_CHECKLIST_ITEM,
    };
    let branch_isolation = branch_isolation_instruction();
    let instructions = format!(
        r#"# Worker Agent

Implement an approved plan by writing code and progressing checklist items.

## Checklist: Your Work Memory

The checklist is your persistent memory for this task. It survives across sessions and tells you exactly where to continue if the work is interrupted.

**Key principles:**
- Start by using `{GET_CHECKLIST}` to read the current checklist — it tells you exactly where you are in the work.
- If the checklist is empty when you start, use `{INSERT_CHECKLIST_ITEM}` to create it based on the plan. Break the plan into clear, actionable steps.
- Each checklist item should describe a meaningful unit of work (for example: "add unit tests for X", "refactor module Y", "update API to validate Z"). Do NOT use checklist items to record internal or platform tool actions (for example: "call {PUSH_WORK}" or "run {PULL_WORK}").
- Use `{CHECK_CHECKLIST_ITEM}` to mark items as checked (`✓`) when you complete them to record progress.
- Use `{INSERT_CHECKLIST_ITEM}` to add new items during work if you discover additional steps needed.
- Use `{UPDATE_CHECKLIST_ITEM}` to edit item text to refine understanding as you work.
- Use `{DELETE_CHECKLIST_ITEM}` to remove items only if they become unnecessary (keep most items for history). **Note:** You cannot delete checked items—this prevents accidental loss of completed work history.

## Access Model

    You can access the internet and run local commands. Your restrictions:
- Do NOT push code directly — no `git push`, no `gh` write operations. The platform coordinates repository remote actions; do not include submission or remote-write actions as checklist items.
- Do NOT run git clone/pull/fetch directly for setting up work — platform tools can prepare the workspace when available. If you need repository data, use the provided helper tools rather than raw git commands.
- For reading GitHub data: use `git` and `gh` CLI only when no platform tool provides the needed information.
- NEVER use git/gh for writing, pushing, or sending data to GitHub.
- The work repository has remote information controlled by the platform; you must not perform direct remote writes yourself.

## Workspace isolation

    {branch_isolation}

Work autonomously. Do not ask the user for anything unless the task genuinely requires human input.

## Workflow

1. Call `{GET_DESCRIPTION}` to read the task
2. Call `{GET_PLAN}` to retrieve the approved implementation plan (posted by the planner)
3. Call `{GET_CHECKLIST}` to read the implementation steps
4. **If checklist is empty**: Create it using `{INSERT_CHECKLIST_ITEM}` to break down the plan into clear, actionable steps (task-focused items only)
5. Call `{GET_DISCUSSION}` if you need additional context from comments
6. **Focus on one unchecked checklist item during this session**. Assume checked items were completed in previous sessions. In exceptional cases where multiple items logically depend on the same setup and can be done together, you may do more than one, but this should be rare.
7. Use platform-provided workspace setup helper `{PULL_WORK}` to prepare the repository and environment; when working with branches, consult `{GET_PARAM_DESTINATION_BRANCH}` and `{GET_PARAM_WORK_BRANCH}` for branch names if needed.
8. `cd` into the returned path and implement the plan
9. Commit changes locally with clear messages (describe what the change does, why, and reference relevant checklist item)
10. When implementation for an item is complete, mark the item done with `{CHECK_CHECKLIST_ITEM}`, save intermediate results with `{PUSH_WORK}` (which requires all changes to be committed first), and update or insert follow-up items as needed
11. Do not add low-level platform or tool-invocation steps (for example, `{PUSH_WORK}`) into your checklist — checklist items should remain human-meaningful and task-focused
12. If you need human clarification or intervention, call `{ASK_USER}` or `{ASK_PLANNER}` as appropriate; use `{REPORT_ERROR}` only to report technical errors
13. Call `{REPORT_RESULTS}` to provide a brief and concise report of your work and finish the session. This report is critical context for further agent calls, so it MUST be compact."#,
    );

    instructions
}

/// Generate hardcoded reviewer instructions using tool name constants.
pub fn reviewer_instructions() -> String {
    use reviewer_tools::{
        GET_DESCRIPTION, GET_PLAN, INSERT_CHECKLIST_ITEM, PULL_WORK, REPORT_ERROR, REPORT_RESULTS,
    };
    let instructions = format!(
        r#"# Reviewer Agent

Review the implementation changes and ensure they meet coding standards and task requirements.

## Access Model

    You have read-only access to the task description and plan, and access to the repository for inspection:
    - Use `{GET_DESCRIPTION}` to understand the original task
    - Use `{GET_PLAN}` to see the implementation plan
    - Use `{PULL_WORK}` to access the work repository and examine changes
    - Use `{INSERT_CHECKLIST_ITEM}` to add a checklist item describing any issues you find
    - Use `{REPORT_ERROR}` only to report technical errors

## Workflow

1. Call `{GET_DESCRIPTION}` to understand the task requirements
2. Call `{GET_PLAN}` to see the agreed implementation
3. Set up the repository using `{PULL_WORK}` and inspect the changes
4. For each issue found, call `{INSERT_CHECKLIST_ITEM}` with a clear title and description explaining the problem and suggested fix
5. Call `{REPORT_RESULTS}` to provide a brief and concise report of your work and finish the session. This report takes part in the context for further agent calls, so it MUST be compact.
"#,
    );

    instructions
}

/// Generate hardcoded merger instructions using tool name constants.
pub fn merger_instructions() -> String {
    use merger_tools::{
        ASK_USER, GET_DESCRIPTION, GET_DISCUSSION, PULL_WORK, PUSH_WORK, REPORT_ERROR,
        REPORT_RESULTS,
    };
    let branch_isolation = branch_isolation_instruction();
    let instructions = format!(
        r#"# Merger Agent

Resolve merge conflicts when the destination branch cannot be automatically merged into the work branch.

## When Merger Runs

A merge conflict occurred when trying to automatically merge the destination branch into the work branch. The merger agent is started to examine conflicts and resolve them when possible.

## Access Model

You have read access to the task and repository:
- Use `{GET_DESCRIPTION}` to understand the task context
- Use `{GET_DISCUSSION}` to see prior comments and what happened
- Use `{PULL_WORK}` to access the repository with conflicts
- Use `{PUSH_WORK}` to push resolved conflicts back to work branch
- Use `{ASK_USER}` to ask the user for clarification on conflict resolution
- Use `{REPORT_ERROR}` to report when conflicts cannot be resolved

## Workspace isolation

    {branch_isolation}

## Workflow

1. Call `{GET_DESCRIPTION}` to understand the task being worked on
2. Call `{GET_DISCUSSION}` for context about what work was being done
3. Call `{PULL_WORK}` to access the repository (the work branch currently has merge conflicts)
4. `cd` into the returned path and examine the conflicts:
   - `git status` to see which files have conflicts
   - `git diff` to examine conflict markers and understand what changed in each branch
   - Review the code in both branches to understand the intent
5. **Attempt automatic resolution:**
   - For simple, non-overlapping changes (e.g., formatting, imports, unrelated edits), apply manual fixes that combine both changes
   - Use `git add` to resolve simple conflicts, then `git commit -m "chore: merge conflicts resolved"`
   - If you can create a reasonable merged version, do so and commit it
6. **If automatic resolution is not possible:**
   - Use `{ASK_USER}` to describe the conflicts and ask which version should be preferred, or ask for guidance
   - Wait for user input before proceeding
7. **After successful resolution:**
   - Commit all changes: `git commit -m "chore: merge resolution"`
   - Use `{PUSH_WORK}` to push the resolved merge to the work branch
   - The task will then resume normally with the merged code

## Conflict Resolution Principles

- Combine non-overlapping changes from both branches (destination and work) when possible
- For conflicting edits to the same code, ask the user which version is preferred
- Preserve the intent of both branches' changes if both changes are valid
- Do NOT delete either branch's work without explicit user guidance
8. Call `{REPORT_RESULTS}` to provide a brief and concise report of your work and finish the session. This report is critical context for further agent calls, so it MUST be compact.
"#,
    );

    instructions
}

/// Generate concise API documentation from a tool router
fn generate_api_docs_from_router<T: Send + Sync + 'static>(
    router: &ToolRouter<T>,
    role_name: &str,
) -> String {
    let tools = router.list_all();

    let mut doc = format!("## {} MCP API\n\n", role_name);
    doc.push_str("Available tools (all pre-scoped to your task):\n\n");

    for tool in tools {
        doc.push_str(&format!("### `{}`\n\n", tool.name));
        doc.push_str(&format!(
            "{}\n\n",
            tool.description.as_deref().unwrap_or("No description")
        ));

        // Parameters
        let schema = &tool.input_schema;
        let properties_obj = schema.get("properties").and_then(|v: &Value| v.as_object());

        if let Some(properties) = properties_obj {
            if !properties.is_empty() {
                doc.push_str("**Parameters:**\n");
                for (name, prop_val) in properties {
                    let required_arr = schema.get("required").and_then(|v: &Value| v.as_array());
                    let required = required_arr
                        .map(|arr| {
                            arr.iter()
                                .any(|v: &Value| v.as_str() == Some(name.as_str()))
                        })
                        .unwrap_or(false);
                    let desc = match prop_val.get("description") {
                        Some(v) => v.as_str().unwrap_or(""),
                        None => "",
                    };
                    let type_str = match prop_val.get("type") {
                        Some(v) => v.as_str().unwrap_or("any"),
                        None => "any",
                    };
                    doc.push_str(&format!(
                        "- `{}` ({}{}) - {}\n",
                        name,
                        type_str,
                        if required { ", required" } else { "" },
                        desc
                    ));
                }
                doc.push('\n');
            } else {
                doc.push_str("**Parameters:** None\n\n");
            }
        } else {
            doc.push_str("**Parameters:** None\n\n");
        }

        doc.push_str("---\n\n");
    }

    doc
}

// -- MCP Trait Hierarchy --

/// Common trait for MCP services (Planner, Worker) - shared implementations
#[allow(async_fn_in_trait)]
pub trait CommonMcpImpl: Send + Sync {
    fn session(&self) -> &TaskSession;
    fn role(&self) -> Role;

    fn role_name(&self) -> &'static str {
        self.role().as_str()
    }

    async fn get_description_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_description",
            self.role_name(),
            self.session().task_id()
        );
        match self.session().get_description().await {
            Ok(desc) => desc,
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn get_discussion_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_discussion",
            self.role_name(),
            self.session().task_id()
        );
        match self.session().get_discussion().await {
            Ok(msgs) => {
                if msgs.is_empty() {
                    "No messages yet.".to_string()
                } else {
                    msgs.join("\n\n---\n\n")
                }
            }
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn report_error_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] report_error",
            self.role_name(),
            self.session().task_id()
        );
        let hostname = get_hostname();

        if let Err(e) = self
            .session()
            .post_message(message, "error", &hostname)
            .await
        {
            tracing::error!(
                "Failed to post error message for task {}: {e}",
                self.session().task_id()
            );
            return format!("Error posting error message: {e}");
        }

        // Signal to pause task processing and wait for user response
        if let Err(e) = self.session().set_signal(crate::Signal::GoAsk).await {
            tracing::error!(
                "Failed to set signal GoAsk for task {} after reporting error: {e}",
                self.session().task_id()
            );
            return format!("Error reporting error but error pausing task: {e}");
        }

        "Error reported to user - task paused pending response".to_string()
    }

    async fn report_results_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] report_results",
            self.role_name(),
            self.session().task_id()
        );
        let hostname = get_hostname();

        if let Err(e) = self
            .session()
            .post_message(message, self.role().as_str(), &hostname)
            .await
        {
            tracing::error!(
                "Failed to post results message for task {}: {e}",
                self.session().task_id()
            );
            return format!("Error posting results message: {e}");
        }

        "Results reported successfully".to_string()
    }

    async fn get_plan_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_plan",
            self.role_name(),
            self.session().task_id()
        );
        match self.session().get_plan().await {
            Ok(plan) => plan,
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn get_checklist_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_checklist",
            self.role_name(),
            self.session().task_id()
        );
        match self.session().get_checklist().await {
            Ok(items) => match serde_json::to_string_pretty(&items) {
                Ok(json) => json,
                Err(e) => format!("Error serializing checklist: {e}"),
            },
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn check_checklist_item_impl(&self, id: &str, checked: bool) -> String {
        tracing::info!(
            "[{}#{}] check_checklist_item id={} checked={}",
            self.role_name(),
            self.session().task_id(),
            id,
            checked
        );
        let item_id = id.to_string();
        match self
            .session()
            .modify_task(move |task| {
                if let Some(item) = task.checklist.iter_mut().find(|item| item.id == item_id) {
                    item.checked = checked;
                }
            })
            .await
        {
            Ok(()) => {
                // Checklist item state updated; signal transitions are handled by
                // the main/run loop after a role session completes. Do not set
                // task signal here to avoid racing state transitions.
                format!(
                    "Checklist item '{}' checked state updated to {}",
                    id, checked
                )
            }
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn insert_checklist_item_impl(
        &self,
        id: &str,
        after_id: Option<String>,
        text: &str,
    ) -> String {
        tracing::info!(
            "[{}#{}] insert_checklist_item id={} after_id={:?}",
            self.role_name(),
            self.session().task_id(),
            id,
            after_id
        );
        let item_id = id.to_string();
        let item_text = text.to_string();
        let after = after_id.clone();

        // Validate first by reading the task
        match self.session().get_checklist().await {
            Ok(items) => {
                if items.iter().any(|item| item.id == item_id) {
                    return format!("Error: Checklist item with id '{}' already exists", id);
                }
                if let Some(ref aid) = after
                    && !items.iter().any(|item| item.id == *aid)
                {
                    return format!("Error: Checklist item with id '{}' not found", aid);
                }
            }
            Err(e) => return format!("Error: {e}"),
        }

        match self
            .session()
            .modify_task(move |task| {
                let new_item = ChecklistItem {
                    id: item_id,
                    checked: false,
                    text: item_text,
                };

                if let Some(ref after_id) = after {
                    if let Some(pos) = task.checklist.iter().position(|item| item.id == *after_id) {
                        task.checklist.insert(pos + 1, new_item);
                    } else {
                        task.checklist.push(new_item);
                    }
                } else {
                    task.checklist.push(new_item);
                }
            })
            .await
        {
            Ok(()) => format!("Checklist item '{}' inserted", id),
            Err(e) => format!("Error updating task: {e}"),
        }
    }

    async fn update_checklist_item_impl(&self, id: &str, text: &str) -> String {
        tracing::info!(
            "[{}#{}] update_checklist_item id={}",
            self.role_name(),
            self.session().task_id(),
            id
        );
        let item_id = id.to_string();
        let item_text = text.to_string();
        match self
            .session()
            .modify_task(move |task| {
                if let Some(item) = task.checklist.iter_mut().find(|item| item.id == item_id) {
                    item.text = item_text;
                }
            })
            .await
        {
            Ok(()) => format!("Checklist item '{}' updated", id),
            Err(e) => format!("Error updating task: {e}"),
        }
    }

    async fn delete_checklist_item_impl(&self, id: &str) -> String {
        tracing::info!(
            "[{}#{}] delete_checklist_item id={}",
            self.role_name(),
            self.session().task_id(),
            id
        );
        let item_id = id.to_string();

        // Pre-validate: check the item exists and is not checked
        match self.session().get_checklist().await {
            Ok(items) => {
                if let Some(item) = items.iter().find(|i| i.id == item_id) {
                    if item.checked {
                        return format!(
                            "Error: Cannot delete checked checklist item '{}'. Checked items are preserved as work history.",
                            id
                        );
                    }
                } else {
                    return format!("Error: Checklist item with id '{}' not found", id);
                }
            }
            Err(e) => return format!("Error: {e}"),
        }

        match self
            .session()
            .modify_task(move |task| {
                task.checklist.retain(|item| item.id != item_id);
            })
            .await
        {
            Ok(()) => format!("Checklist item '{}' deleted", id),
            Err(e) => format!("Error updating task: {e}"),
        }
    }

    async fn get_param_impl(&self, param: Parameter) -> String {
        tracing::info!(
            "[{}#{}] get_param_{}",
            self.role_name(),
            self.session().task_id(),
            param.name()
        );
        match self.session().get_parameter(param).await {
            Ok(Some(value)) => value,
            Ok(None) => format!("{} is not set", param.name()),
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn set_param_impl(&self, param: Parameter, value: Option<String>) -> String {
        tracing::info!(
            "[{}#{}] set_param_{} value={:?}",
            self.role_name(),
            self.session().task_id(),
            param.name(),
            value
        );
        match self.session().set_parameter(param, value).await {
            Ok(()) => format!("{} updated", param.name()),
            Err(e) => format!("Error: {e}"),
        }
    }
}

/// Preparator-specific MCP implementations
#[allow(async_fn_in_trait)]
pub trait PreparatorMcpImpl: CommonMcpImpl {
    async fn get_param_destination_repository_impl(&self) -> String {
        self.get_param_impl(Parameter::DestinationRepository).await
    }

    async fn set_param_destination_repository_impl(&self, value: Option<String>) -> String {
        self.set_param_impl(Parameter::DestinationRepository, value)
            .await
    }

    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::DestinationBranch).await
    }

    async fn set_param_destination_branch_impl(&self, value: Option<String>) -> String {
        self.set_param_impl(Parameter::DestinationBranch, value)
            .await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::WorkBranch).await
    }

    async fn set_param_work_branch_postfix_impl(&self, value: Option<String>) -> String {
        let branch = value.map(|v| self.session().create_branch_name(&v));
        self.set_param_impl(Parameter::WorkBranch, branch).await
    }
}

/// Planner-specific MCP implementations
#[allow(async_fn_in_trait)]
pub trait PlannerMcpImpl: CommonMcpImpl {
    async fn post_plan_impl(&self, plan: &str) -> String {
        tracing::info!("[planner#{}] post_plan", self.session().task_id());
        let plan_text = plan.to_string();
        match self
            .session()
            .modify_task(move |task| {
                task.plan = plan_text;
            })
            .await
        {
            Ok(()) => {
                // Mark plan as ready for worker to implement
                if let Err(e) = self.session().set_signal(crate::Signal::GoWork).await {
                    tracing::error!(
                        "Failed to set signal GoWork for task {} after posting plan: {e}",
                        self.session().task_id()
                    );
                    return format!("Plan posted but error marking task ready for work: {e}");
                }
                "Plan posted and task ready for worker implementation".to_string()
            }
            Err(e) => format!("Error updating task: {e}"),
        }
    }

    async fn pull_work_impl(&self) -> String {
        tracing::info!("[planner#{}] pull_work", self.session().task_id());
        match self.session().pull_work().await {
            Ok(path) => path,
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::DestinationBranch).await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::WorkBranch).await
    }
}

/// Worker-specific MCP implementations
#[allow(async_fn_in_trait)]
pub trait WorkerMcpImpl: CommonMcpImpl {
    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::DestinationBranch).await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::WorkBranch).await
    }

    async fn ask_user_impl(&self, message: &str) -> String {
        tracing::info!("[worker#{}] ask_user", self.session().task_id());
        let hostname = get_hostname();

        if let Err(e) = self
            .session()
            .post_message(message, self.role().as_str(), &hostname)
            .await
        {
            tracing::error!(
                "Failed to post worker message for task {}: {e}",
                self.session().task_id()
            );
            return format!("Error posting message: {e}");
        }

        // Signal to pause task processing and wait for user response
        if let Err(e) = self.session().set_signal(crate::Signal::GoAsk).await {
            tracing::error!(
                "Failed to set signal GoAsk for task {} after ask_user: {e}",
                self.session().task_id()
            );
            return format!("Question posted but error pausing task: {e}");
        }
        "Message posted to user - task paused pending response".to_string()
    }

    async fn ask_planner_impl(&self, message: &str) -> String {
        tracing::info!("[worker#{}] ask_planner", self.session().task_id());
        let hostname = get_hostname();

        if let Err(e) = self
            .session()
            .post_message(message, self.role().as_str(), &hostname)
            .await
        {
            tracing::error!(
                "Failed to post worker->planner message for task {}: {e}",
                self.session().task_id()
            );
            return format!("Error posting message: {e}");
        }

        // Pass task back to planner agent for clarification or re-planning
        if let Err(e) = self.session().set_signal(crate::Signal::GoPlan).await {
            tracing::error!(
                "Failed to set signal GoPlan for task {} after ask_planner: {e}",
                self.session().task_id()
            );
            return format!("Message posted but error returning to planner: {e}");
        }
        "Message posted to planner - task returned for clarification".to_string()
    }

    async fn pull_work_impl(&self) -> String {
        tracing::info!("[worker#{}] pull_work", self.session().task_id());
        match self.session().pull_work().await {
            Ok(path) => path,
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn push_work_impl(&self) -> String {
        tracing::info!("[worker#{}] push_work", self.session().task_id());
        match self.session().push_work().await {
            Ok(()) => "Work branch pushed successfully".to_string(),
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn mark_done_impl(&self) -> String {
        tracing::info!("[worker#{}] mark_done", self.session().task_id());
        match self.session().mark_done().await {
            Ok(()) => "Task marked as done".to_string(),
            Err(e) => format!("Error: {e}"),
        }
    }
}

/// Reviewer-specific MCP implementations
#[allow(async_fn_in_trait)]
pub trait ReviewerMcpImpl: CommonMcpImpl {
    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::DestinationBranch).await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::WorkBranch).await
    }

    async fn pull_work_impl(&self) -> String {
        tracing::info!("[reviewer#{}] pull_work", self.session().task_id());
        match self.session().pull_work().await {
            Ok(path) => path,
            Err(e) => format!("Error: {e}"),
        }
    }
}

// -- Merger MCP service --

pub trait MergerMcpImpl: CommonMcpImpl {
    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::DestinationBranch).await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::WorkBranch).await
    }

    async fn pull_work_impl(&self) -> String {
        tracing::info!("[merger#{}] pull_work", self.session().task_id());
        match self.session().pull_work().await {
            Ok(path) => path,
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn push_work_impl(&self) -> String {
        tracing::info!("[merger#{}] push_work", self.session().task_id());
        match self.session().push_work().await {
            Ok(()) => "Merged conflicts pushed successfully".to_string(),
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn ask_user_impl(&self, message: &str) -> String {
        tracing::info!("[merger#{}] ask_user", self.session().task_id());
        let hostname = get_hostname();

        if let Err(e) = self
            .session()
            .post_message(message, self.role().as_str(), &hostname)
            .await
        {
            tracing::error!(
                "Failed to post merger message for task {}: {e}",
                self.session().task_id()
            );
            return format!("Error posting message: {e}");
        }

        // Signal to pause task processing and wait for user response
        if let Err(e) = self.session().set_signal(crate::Signal::GoAsk).await {
            tracing::error!(
                "Failed to set signal GoAsk for task {} after asking user: {e}",
                self.session().task_id()
            );
            return format!("Error pausing task: {e}");
        }

        "Message posted to user - task paused pending response".to_string()
    }
}

#[derive(Clone)]
pub struct MergerMcp {
    session: TaskSession,
    tool_router: ToolRouter<Self>,
}

impl CommonMcpImpl for MergerMcp {
    fn session(&self) -> &TaskSession {
        &self.session
    }

    fn role(&self) -> Role {
        Role::Merger
    }
}

impl MergerMcpImpl for MergerMcp {}

#[tool_router]
impl MergerMcp {
    pub fn new(zbobr: Zbobr, task_id: u64) -> Self {
        Self {
            session: zbobr.task_session(task_id),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get the current description for this task")]
    async fn get_description(&self) -> String {
        self.get_description_impl().await
    }

    #[tool(description = "Get all discussion messages on this task")]
    async fn get_discussion(&self) -> String {
        self.get_discussion_impl().await
    }

    #[tool(description = "Report an error to the user and pause task processing")]
    async fn report_error(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_error_impl(&params.message).await
    }

    #[tool(
        description = "Post a message to the user and pause task processing until user responds"
    )]
    async fn ask_user(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.ask_user_impl(&params.message).await
    }

    #[tool(
        description = "Your workspace currently has a merge conflict. Use this to access the repository and resolve the conflicts"
    )]
    async fn pull_work(&self) -> String {
        self.pull_work_impl().await
    }

    #[tool(
        description = "Push the resolved merge to the work branch in the fork. All changes must be committed before pushing."
    )]
    async fn push_work(&self) -> String {
        self.push_work_impl().await
    }

    #[tool(description = "Get the destination branch name for this task (read-only)")]
    async fn get_param_destination_branch(&self) -> String {
        self.get_param_destination_branch_impl().await
    }

    #[tool(description = "Get the work branch name for this task (read-only)")]
    async fn get_param_work_branch(&self) -> String {
        self.get_param_work_branch_impl().await
    }

    #[tool(
        description = "Provide a brief and concise report of your results and finish your work. These reports add up to discussion and shorten the context for further agent calls, so they MUST be compact."
    )]
    async fn report_results(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_results_impl(&params.message).await
    }
}

#[tool_handler]
impl ServerHandler for MergerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Merger tools: resolve merge conflicts in the work branch.".to_string(),
            ),
            ..Default::default()
        }
    }
}

impl MergerMcp {
    /// Generate API documentation for merger tools
    pub fn generate_api_docs() -> String {
        let tools = Self::tool_router();
        generate_api_docs_from_router(&tools, "Merger")
    }
}

// -- Preparator MCP service --

#[derive(Clone)]
pub struct PreparatorMcp {
    session: TaskSession,
    tool_router: ToolRouter<Self>,
}

impl CommonMcpImpl for PreparatorMcp {
    fn session(&self) -> &TaskSession {
        &self.session
    }

    fn role(&self) -> Role {
        Role::Preparator
    }
}

impl PreparatorMcpImpl for PreparatorMcp {}

#[tool_router]
impl PreparatorMcp {
    pub fn new(zbobr: Zbobr, task_id: u64) -> Self {
        Self {
            session: zbobr.task_session(task_id),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get the current description for this task (read-only)")]
    async fn get_description(&self) -> String {
        self.get_description_impl().await
    }

    #[tool(description = "Get all discussion messages on this task")]
    async fn get_discussion(&self) -> String {
        self.get_discussion_impl().await
    }

    #[tool(description = "Report an error to the user and pause task processing")]
    async fn report_error(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_error_impl(&params.message).await
    }

    #[tool(description = "Get the destination repository URL for this task (read-only)")]
    async fn get_param_destination_repository(&self) -> String {
        self.get_param_destination_repository_impl().await
    }

    #[tool(
        description = "Set the destination repository for this task (full git URL, local path, or 'owner/repo')"
    )]
    async fn set_param_destination_repository(
        &self,
        Parameters(params): Parameters<SetDestinationRepositoryParam>,
    ) -> String {
        self.set_param_destination_repository_impl(params.value)
            .await
    }

    #[tool(description = "Get the destination branch name for this task (read-only)")]
    async fn get_param_destination_branch(&self) -> String {
        self.get_param_destination_branch_impl().await
    }

    #[tool(description = "Set the destination branch name for this task (e.g. 'main')")]
    async fn set_param_destination_branch(
        &self,
        Parameters(params): Parameters<SetDestinationBranchParam>,
    ) -> String {
        self.set_param_destination_branch_impl(params.value).await
    }

    #[tool(
        description = "Set the work branch postfix for this task (the postfix segment, e.g. 'implement-feature')"
    )]
    async fn set_param_work_branch_postfix(
        &self,
        Parameters(params): Parameters<SetDestinationBranchParam>,
    ) -> String {
        self.set_param_work_branch_postfix_impl(params.value).await
    }

    #[tool(description = "Get the work branch name for this task (read-only)")]
    async fn get_param_work_branch(&self) -> String {
        self.get_param_work_branch_impl().await
    }

    #[tool(
        description = "Provide a brief and concise report of your results and finish your work. These reports add up to discussion and shorten the context for further agent calls, so they MUST be compact."
    )]
    async fn report_results(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_results_impl(&params.message).await
    }
}

#[tool_handler]
impl ServerHandler for PreparatorMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Preparator tools: read task description and set implementation parameters."
                    .to_string(),
            ),
            ..Default::default()
        }
    }
}

impl PreparatorMcp {
    /// Generate API documentation for preparator tools
    pub fn generate_api_docs() -> String {
        let tools = Self::tool_router();
        generate_api_docs_from_router(&tools, "Preparator")
    }
}

// -- Planner MCP service --

#[derive(Clone)]
pub struct PlannerMcp {
    session: TaskSession,
    tool_router: ToolRouter<Self>,
}

impl CommonMcpImpl for PlannerMcp {
    fn session(&self) -> &TaskSession {
        &self.session
    }

    fn role(&self) -> Role {
        Role::Planner
    }
}

impl PlannerMcpImpl for PlannerMcp {}

#[tool_router]
impl PlannerMcp {
    pub fn new(zbobr: Zbobr, task_id: u64) -> Self {
        Self {
            session: zbobr.task_session(task_id),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get the current description for this task (read-only)")]
    async fn get_description(&self) -> String {
        self.get_description_impl().await
    }

    #[tool(description = "Get all discussion messages on this task")]
    async fn get_discussion(&self) -> String {
        self.get_discussion_impl().await
    }

    #[tool(description = "Report an error to the user and pause task processing")]
    async fn report_error(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_error_impl(&params.message).await
    }

    #[tool(description = "Get the current implementation plan for this task")]
    async fn get_plan(&self) -> String {
        self.get_plan_impl().await
    }

    #[tool(description = "Post or replace the implementation plan for this task")]
    async fn post_plan(&self, Parameters(params): Parameters<DescriptionParam>) -> String {
        self.post_plan_impl(&params.description).await
    }

    #[tool(
        description = "Clone the fork of the destination_repository and return the path. Automatically sets the current branch to work_branch (created from destination_branch). Stashes local changes if a different branch is selected as current. The work repository has all remote information cleared - only pull_work and push_work know where to push/pull. The model must not do git push directly."
    )]
    async fn pull_work(&self) -> String {
        self.pull_work_impl().await
    }

    #[tool(description = "Get the destination branch name for this task (read-only)")]
    async fn get_param_destination_branch(&self) -> String {
        self.get_param_destination_branch_impl().await
    }

    #[tool(description = "Get the work branch name for this task (read-only)")]
    async fn get_param_work_branch(&self) -> String {
        self.get_param_work_branch_impl().await
    }

    #[tool(
        description = "Provide a brief and concise report of your results and finish your work. These reports add up to discussion and shorten the context for further agent calls, so they MUST be compact."
    )]
    async fn report_results(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_results_impl(&params.message).await
    }
}

#[tool_handler]
impl ServerHandler for PlannerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Planner tools: investigate task and create implementation plan.".to_string(),
            ),
            ..Default::default()
        }
    }
}

impl PlannerMcp {
    /// Generate API documentation for planner tools
    pub fn generate_api_docs() -> String {
        let tools = Self::tool_router();
        generate_api_docs_from_router(&tools, "Planner")
    }
}

// -- Worker MCP service --

#[derive(Clone)]
pub struct WorkerMcp {
    session: TaskSession,
    tool_router: ToolRouter<Self>,
}

impl CommonMcpImpl for WorkerMcp {
    fn session(&self) -> &TaskSession {
        &self.session
    }

    fn role(&self) -> Role {
        Role::Worker
    }
}

impl WorkerMcpImpl for WorkerMcp {}

#[tool_router]
impl WorkerMcp {
    pub fn new(zbobr: Zbobr, task_id: u64) -> Self {
        Self {
            session: zbobr.task_session(task_id),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get the current description for this task")]
    async fn get_description(&self) -> String {
        self.get_description_impl().await
    }

    #[tool(description = "Get all discussion messages on this task")]
    async fn get_discussion(&self) -> String {
        self.get_discussion_impl().await
    }

    #[tool(description = "Report an error to the user and pause task processing")]
    async fn report_error(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_error_impl(&params.message).await
    }

    #[tool(
        description = "Post a message to the user and pause task processing until user responds"
    )]
    async fn ask_user(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.ask_user_impl(&params.message).await
    }

    #[tool(
        description = "Post a message to the planner and pass the task back for clarification or re-planning"
    )]
    async fn ask_planner(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.ask_planner_impl(&params.message).await
    }

    #[tool(description = "Get the current implementation plan for this task")]
    async fn get_plan(&self) -> String {
        self.get_plan_impl().await
    }

    #[tool(
        description = "Clone the fork of the destination_repository and return the path. Automatically sets the current branch to work_branch (created from destination_branch). Stashes local changes if a different branch is selected as current. The work repository has all remote information cleared - only pull_work and push_work know where to push/pull. The model must not do git push directly."
    )]
    async fn pull_work(&self) -> String {
        self.pull_work_impl().await
    }

    #[tool(
        description = "Push the work_branch in the cloned repository. Returns nothing. Stashes local changes if a different branch is selected as current. All changes must be committed before pushing - the push will fail with an error listing uncommitted files if any exist. The work repository has all remote information cleared - only pull_work and push_work know where to push/pull. The model must not do git push directly."
    )]
    async fn push_work(&self) -> String {
        self.push_work_impl().await
    }

    #[tool(description = "Get the task checklist as a list of checkbox items")]
    async fn get_checklist(&self) -> String {
        self.get_checklist_impl().await
    }

    #[tool(description = "Insert a new checklist item (always created in unchecked state)")]
    async fn insert_checklist_item(
        &self,
        Parameters(params): Parameters<InsertChecklistItemParam>,
    ) -> String {
        self.insert_checklist_item_impl(&params.id, params.after_id.clone(), &params.text)
            .await
    }

    #[tool(description = "Update a checklist item's text")]
    async fn update_checklist_item(
        &self,
        Parameters(params): Parameters<UpdateChecklistItemParam>,
    ) -> String {
        self.update_checklist_item_impl(&params.id, &params.text)
            .await
    }

    #[tool(description = "Check or uncheck a checklist item")]
    async fn check_checklist_item(
        &self,
        Parameters(params): Parameters<CheckChecklistItemParam>,
    ) -> String {
        self.check_checklist_item_impl(&params.id, params.checked)
            .await
    }

    #[tool(
        description = "Delete an unchecked checklist item (checked items are preserved as history)"
    )]
    async fn delete_checklist_item(
        &self,
        Parameters(params): Parameters<DeleteChecklistItemParam>,
    ) -> String {
        self.delete_checklist_item_impl(&params.id).await
    }

    #[tool(description = "Get the destination branch name for this task (read-only)")]
    async fn get_param_destination_branch(&self) -> String {
        self.get_param_destination_branch_impl().await
    }

    #[tool(description = "Get the work branch name for this task (read-only)")]
    async fn get_param_work_branch(&self) -> String {
        self.get_param_work_branch_impl().await
    }

    #[tool(
        description = "Provide a brief and concise report of your results and finish your work. These reports add up to discussion and shorten the context for further agent calls, so they MUST be compact."
    )]
    async fn report_results(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_results_impl(&params.message).await
    }
}

#[tool_handler]
impl ServerHandler for WorkerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Worker tools: implement task according to plan, submit work, mark done."
                    .to_string(),
            ),
            ..Default::default()
        }
    }
}

impl WorkerMcp {
    /// Generate API documentation for worker tools
    pub fn generate_api_docs() -> String {
        let tools = Self::tool_router();
        generate_api_docs_from_router(&tools, "Worker")
    }
}

// -- Reviewer MCP service --

#[derive(Clone)]
pub struct ReviewerMcp {
    session: TaskSession,
    tool_router: ToolRouter<Self>,
}

impl CommonMcpImpl for ReviewerMcp {
    fn session(&self) -> &TaskSession {
        &self.session
    }

    fn role(&self) -> Role {
        Role::Reviewer
    }
}

impl ReviewerMcpImpl for ReviewerMcp {}

#[tool_router]
impl ReviewerMcp {
    pub fn new(zbobr: Zbobr, task_id: u64) -> Self {
        Self {
            session: zbobr.task_session(task_id),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get the current description for this task (read-only)")]
    async fn get_description(&self) -> String {
        self.get_description_impl().await
    }

    #[tool(description = "Get all discussion messages on this task")]
    async fn get_discussion(&self) -> String {
        self.get_discussion_impl().await
    }

    #[tool(description = "Report an error to the user and pause task processing")]
    async fn report_error(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_error_impl(&params.message).await
    }

    #[tool(description = "Get the current implementation plan for this task (read-only)")]
    async fn get_plan(&self) -> String {
        self.get_plan_impl().await
    }

    #[tool(
        description = "Clone the fork of the destination_repository and return the path. Automatically sets the current branch to work_branch (created from destination_branch). Read-only access for review purposes."
    )]
    async fn pull_work(&self) -> String {
        self.pull_work_impl().await
    }

    #[tool(description = "Get the task checklist as a list of checkbox items")]
    async fn get_checklist(&self) -> String {
        self.get_checklist_impl().await
    }

    #[tool(
        description = "Insert a new checklist item for review remarks (always created in unchecked state)"
    )]
    async fn insert_checklist_item(
        &self,
        Parameters(params): Parameters<InsertChecklistItemParam>,
    ) -> String {
        self.insert_checklist_item_impl(&params.id, params.after_id.clone(), &params.text)
            .await
    }

    #[tool(description = "Update a checklist item's text")]
    async fn update_checklist_item(
        &self,
        Parameters(params): Parameters<UpdateChecklistItemParam>,
    ) -> String {
        self.update_checklist_item_impl(&params.id, &params.text)
            .await
    }

    #[tool(description = "Check or uncheck a checklist item")]
    async fn check_checklist_item(
        &self,
        Parameters(params): Parameters<CheckChecklistItemParam>,
    ) -> String {
        self.check_checklist_item_impl(&params.id, params.checked)
            .await
    }

    #[tool(
        description = "Delete an unchecked checklist item (checked items are preserved as history)"
    )]
    async fn delete_checklist_item(
        &self,
        Parameters(params): Parameters<DeleteChecklistItemParam>,
    ) -> String {
        self.delete_checklist_item_impl(&params.id).await
    }

    #[tool(description = "Get the destination branch name for this task (read-only)")]
    async fn get_param_destination_branch(&self) -> String {
        self.get_param_destination_branch_impl().await
    }

    #[tool(description = "Get the work branch name for this task (read-only)")]
    async fn get_param_work_branch(&self) -> String {
        self.get_param_work_branch_impl().await
    }

    #[tool(
        description = "Provide a brief and concise report of your results and finish your work. These reports add up to discussion and shorten the context for further agent calls, so they MUST be compact."
    )]
    async fn report_results(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_results_impl(&params.message).await
    }
}

#[tool_handler]
impl ServerHandler for ReviewerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Reviewer tools: review implementation changes, add review remarks to checklist."
                    .to_string(),
            ),
            ..Default::default()
        }
    }
}

impl ReviewerMcp {
    /// Generate API documentation for reviewer tools
    pub fn generate_api_docs() -> String {
        let tools = Self::tool_router();
        generate_api_docs_from_router(&tools, "Reviewer")
    }
}

/// Find an available port starting from the given base port.
/// Tries ports incrementally until one is available.
async fn find_available_port(base_port: u16) -> anyhow::Result<u16> {
    for port in base_port..=base_port + 100 {
        match tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await {
            Ok(_) => return Ok(port),
            Err(_) => continue,
        }
    }
    anyhow::bail!(
        "Could not find available port in range {base_port}..{}",
        base_port + 100
    )
}

async fn serve_mcp(base_port: u16, path: &str, router: axum::Router) -> anyhow::Result<u16> {
    // Find an available port starting from base_port
    let port = find_available_port(base_port).await?;

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    tracing::info!("MCP server listening on http://127.0.0.1:{port}{path}");

    // Spawn the actual server in a background task
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router)
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c().await.ok();
            })
            .await
        {
            tracing::error!("Axum server error: {e}");
        }
    });

    Ok(port)
}

/// Run the MCP HTTP server scoped to a role (planner or worker) and task.
/// Returns the actual port that was assigned (spawns server in background).
pub async fn run_role_mcp_server(
    zbobr: Zbobr,
    base_port: u16,
    role: Role,
    task_id: u64,
) -> anyhow::Result<u16> {
    let path = format!("/{}/{}", role, task_id);

    let router = match role {
        Role::Preparator => {
            tracing::info!("Creating PreparatorMcp service for task {task_id} at path {path}");
            let svc = StreamableHttpService::new(
                move || {
                    tracing::debug!("Creating new PreparatorMcp instance for task {task_id}");
                    Ok(PreparatorMcp::new(zbobr.clone(), task_id))
                },
                Arc::new(LocalSessionManager::default()),
                Default::default(),
            );
            axum::Router::new().nest_service(&path, svc)
        }
        Role::Planner => {
            tracing::info!("Creating PlannerMcp service for task {task_id} at path {path}");
            let svc = StreamableHttpService::new(
                move || {
                    tracing::debug!("Creating new PlannerMcp instance for task {task_id}");
                    Ok(PlannerMcp::new(zbobr.clone(), task_id))
                },
                Arc::new(LocalSessionManager::default()),
                Default::default(),
            );
            axum::Router::new().nest_service(&path, svc)
        }
        Role::Worker => {
            tracing::info!("Creating WorkerMcp service for task {task_id} at path {path}");
            let svc = StreamableHttpService::new(
                move || {
                    tracing::debug!("Creating new WorkerMcp instance for task {task_id}");
                    Ok(WorkerMcp::new(zbobr.clone(), task_id))
                },
                Arc::new(LocalSessionManager::default()),
                Default::default(),
            );
            axum::Router::new().nest_service(&path, svc)
        }
        Role::Reviewer => {
            tracing::info!("Creating ReviewerMcp service for task {task_id} at path {path}");
            let svc = StreamableHttpService::new(
                move || {
                    tracing::debug!("Creating new ReviewerMcp instance for task {task_id}");
                    Ok(ReviewerMcp::new(zbobr.clone(), task_id))
                },
                Arc::new(LocalSessionManager::default()),
                Default::default(),
            );
            axum::Router::new().nest_service(&path, svc)
        }
        Role::Merger => {
            tracing::info!("Creating MergerMcp service for task {task_id} at path {path}");
            let svc = StreamableHttpService::new(
                move || {
                    tracing::debug!("Creating new MergerMcp instance for task {task_id}");
                    Ok(MergerMcp::new(zbobr.clone(), task_id))
                },
                Arc::new(LocalSessionManager::default()),
                Default::default(),
            );
            axum::Router::new().nest_service(&path, svc)
        }
    };

    serve_mcp(base_port, &path, router).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Model, Tool};

    struct StubTaskBackend;

    #[async_trait::async_trait]
    impl crate::backend::TaskBackend for StubTaskBackend {
        async fn get_task(&self, _id: u64) -> anyhow::Result<crate::Task> {
            unimplemented!()
        }
        async fn create_task(
            &self,
            _title: &str,
            _description: &str,
            _stage: crate::Stage,
            _tool: Option<crate::Tool>,
            _model: Option<crate::Model>,
            _parameters: std::collections::HashMap<crate::Parameter, String>,
        ) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn close_task(&self, _id: u64) -> anyhow::Result<()> {
            unimplemented!()
        }
        async fn is_task_closed(&self, _id: u64) -> anyhow::Result<bool> {
            unimplemented!()
        }
        async fn modify_task(
            &self,
            _id: u64,
            _mutate: Box<dyn FnOnce(crate::Task) -> crate::Task + Send>,
        ) -> anyhow::Result<()> {
            unimplemented!()
        }
        async fn list_tasks_by_stage(
            &self,
            _stage: crate::Stage,
            _tool: Option<crate::Tool>,
        ) -> anyhow::Result<Vec<crate::Task>> {
            unimplemented!()
        }
        async fn get_task_comments(&self, _id: u64) -> anyhow::Result<Vec<String>> {
            unimplemented!()
        }
        async fn post_task_comment(
            &self,
            _id: u64,
            _body: &str,
            _role: &str,
            _hostname: &str,
        ) -> anyhow::Result<()> {
            unimplemented!()
        }
        async fn setup(&self, _force: bool) -> anyhow::Result<()> {
            unimplemented!()
        }
        async fn validate_connectivity(&self) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn debug_state(&self) -> String {
            "StubTaskBackend".to_string()
        }
    }

    struct StubRepoBackend;

    #[async_trait::async_trait]
    impl crate::backend::RepoBackend for StubRepoBackend {
        async fn clone_and_setup(
            &self,
            _repo: &str,
            _branch: &str,
            _workspace_path: &std::path::Path,
        ) -> anyhow::Result<std::path::PathBuf> {
            unimplemented!()
        }
        async fn clone_readonly(
            &self,
            _repo: &str,
            _branch: &str,
            _workspace_path: &std::path::Path,
        ) -> anyhow::Result<std::path::PathBuf> {
            unimplemented!()
        }
        async fn sync_fork(&self, _repo: &str, _branch: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
        async fn setup_fork_remote_and_push(
            &self,
            _work_dir: &std::path::Path,
            _target_repo: &str,
            _work_branch: &str,
        ) -> anyhow::Result<()> {
            unimplemented!()
        }
        async fn push_and_create_pr(
            &self,
            _repo: &str,
            _workspace_path: &std::path::Path,
            _pr_title: &str,
            _pr_body: &str,
        ) -> anyhow::Result<String> {
            unimplemented!()
        }
        async fn create_pr_in_fork(
            &self,
            _repo_name: &str,
            _work_branch: &str,
            _dest_branch: &str,
            _pr_title: &str,
            _pr_body: &str,
        ) -> anyhow::Result<String> {
            unimplemented!()
        }
        async fn parse_pr_to_repo_branch(&self, _pr_ref: &str) -> anyhow::Result<(String, String)> {
            unimplemented!()
        }
        async fn validate_connectivity(&self) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn debug_state(&self) -> String {
            "StubRepoBackend".to_string()
        }
    }

    fn test_config() -> crate::config::ZbobrDispatcherConfig {
        crate::config::ZbobrDispatcherConfig {
            default_model: Model::Gpt4o,
            workspaces: std::path::PathBuf::from("/tmp"),
            agent_github_token: "agent-token".to_string(),
            copilot_github_token: "copilot-token".to_string(),
            backend: crate::config::BackendType::GitHub,
            cli_tool: Tool::Claude,
            preparator_prompts: vec![],
            planner_prompts: vec![],
            worker_prompts: vec![],
            reviewer_prompts: vec![],
            merger_prompts: vec![],
            work_branch_prefix: "zbobr_fix".to_string(),
            prompts_path: None,
            git_user_name: "Test User".to_string(),
            git_user_email: "test@example.com".to_string(),
        }
    }

    fn test_zbobr() -> Zbobr {
        let config = test_config();
        let task_backend: std::sync::Arc<dyn crate::backend::TaskBackend> =
            std::sync::Arc::new(StubTaskBackend);
        let repo_backend: std::sync::Arc<dyn crate::backend::RepoBackend> =
            std::sync::Arc::new(StubRepoBackend);
        Zbobr::new(config, task_backend, repo_backend)
    }

    #[tokio::test]
    async fn test_preparator_tools_consistency() {
        let zbobr = test_zbobr();
        let preparator = PreparatorMcp::new(zbobr, 123);

        let tools = preparator.tool_router.list_all();
        let mut tool_names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();
        tool_names.sort();

        let mut expected_names = preparator_tools::ALL_TOOLS.to_vec();
        expected_names.sort();

        assert_eq!(
            tool_names, expected_names,
            "Exposed preparator tools do not match preparator_tools::ALL_TOOLS"
        );
    }

    #[tokio::test]
    async fn test_planner_tools_consistency() {
        let zbobr = test_zbobr();
        let planner = PlannerMcp::new(zbobr, 123);

        let tools = planner.tool_router.list_all();
        let mut tool_names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();
        tool_names.sort();

        let mut expected_names = planner_tools::ALL_TOOLS.to_vec();
        expected_names.sort();

        assert_eq!(
            tool_names, expected_names,
            "Exposed planner tools do not match planner_tools::ALL_TOOLS"
        );
    }

    #[tokio::test]
    async fn test_worker_tools_consistency() {
        let zbobr = test_zbobr();
        let worker = WorkerMcp::new(zbobr, 123);

        let tools = worker.tool_router.list_all();
        let mut tool_names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();
        tool_names.sort();

        let mut expected_names = worker_tools::ALL_TOOLS.to_vec();
        expected_names.sort();

        assert_eq!(
            tool_names, expected_names,
            "Exposed worker tools do not match worker_tools::ALL_TOOLS"
        );
    }
}
