use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::{
    mcp::{
        common::{
            CheckChecklistItemParam, DeleteChecklistItemParam, GetHistoryParam,
            InsertChecklistItemParam, MessageParam, UpdateChecklistItemParam,
        },
        traits::{CommonMcpImpl, WorkerMcpImpl},
    },
    task::{Model, RoleSession, Tool},
};

#[derive(Clone)]
pub struct WorkerMcp {
    session: RoleSession,
    tool_router: ToolRouter<Self>,
    tool: Tool,
    model: Model,
    stage_name: String,
    transitions: std::collections::HashMap<String, String>,
}

impl CommonMcpImpl for WorkerMcp {
    fn session(&self) -> &RoleSession {
        &self.session
    }

    fn role(&self) -> crate::task::Role {
        crate::task::Role::Worker
    }

    fn mcp_tool(&self) -> Tool {
        self.tool
    }

    fn mcp_model(&self) -> Model {
        self.model.clone()
    }

    fn stage_name(&self) -> &str {
        &self.stage_name
    }

    fn transitions(&self) -> &std::collections::HashMap<String, String> {
        &self.transitions
    }
}

impl WorkerMcpImpl for WorkerMcp {}

#[tool_router]
impl WorkerMcp {
    pub fn new(
        session: RoleSession,
        tool: Tool,
        model: Model,
        stage_name: String,
        transitions: std::collections::HashMap<String, String>,
    ) -> Self {
        Self {
            session,
            tool_router: Self::tool_router(),
            tool,
            model,
            stage_name,
            transitions,
        }
    }

    #[tool(
        description = "Get task history chunk. Optional offset: chunk index (0 = oldest, omitted = latest). Response includes current_chunk and last_chunk for navigation."
    )]
    async fn get_history(&self, Parameters(params): Parameters<GetHistoryParam>) -> String {
        self.get_history_impl(params.offset).await
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
        crate::mcp::common::generate_api_docs_from_router(&tools, "Worker")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tools_match_common_list() {
        let tools = WorkerMcp::tool_router().list_all();
        let mut names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();
        names.sort();
        let mut expected = crate::mcp::worker_tools::ALL_TOOLS.to_vec();
        expected.sort();
        assert_eq!(
            names, expected,
            "worker tool router diverged from common list"
        );
    }
}
