use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::{
    ZbobrDispatcherDyn,
    mcp::{
        common::{
            DeleteChecklistItemParam, DescriptionParam, GetHistoryParam, InsertChecklistItemParam,
            MessageParam, UpdateChecklistItemParam,
        },
        traits::{CommonMcpImpl, PlannerMcpImpl},
    },
    task::{RoleSession, Model, Tool},
};

#[derive(Clone)]
pub struct PlannerMcp {
    session: RoleSession,
    tool_router: ToolRouter<Self>,
    tool: Tool,
    model: Model,
}

impl CommonMcpImpl for PlannerMcp {
    fn session(&self) -> &RoleSession {
        &self.session
    }

    fn role(&self) -> crate::task::Role {
        crate::task::Role::Planner
    }

    fn mcp_tool(&self) -> Tool {
        self.tool
    }

    fn mcp_model(&self) -> Model {
        self.model.clone()
    }
}

impl PlannerMcpImpl for PlannerMcp {}

#[tool_router]
impl PlannerMcp {
    pub fn new(zbobr: ZbobrDispatcherDyn, task_id: u64, tool: Tool, model: Model) -> Self {
        Self {
            session: zbobr.role_session(task_id),
            tool_router: Self::tool_router(),
            tool,
            model,
        }
    }

    #[tool(
        description = "Get task history chunk. Optional offset: chunk index (0 = oldest, omitted = latest). Response includes current_chunk and last_chunk for navigation."
    )]
    async fn get_history(&self, Parameters(params): Parameters<GetHistoryParam>) -> String {
        self.get_history_impl(params.offset).await
    }

    #[tool(description = "Post the implementation plan for this task and finish your session")]
    async fn post_plan(&self, Parameters(params): Parameters<DescriptionParam>) -> String {
        self.post_plan_impl(&params.description).await
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

    #[tool(
        description = "Delete an unchecked checklist item (checked items are preserved as history)"
    )]
    async fn delete_checklist_item(
        &self,
        Parameters(params): Parameters<DeleteChecklistItemParam>,
    ) -> String {
        self.delete_checklist_item_impl(&params.id).await
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

    #[tool(description = "Get the destination branch name for this task (read-only)")]
    async fn get_param_destination_branch(&self) -> String {
        self.get_param_destination_branch_impl().await
    }

    #[tool(description = "Get the work branch name for this task (read-only)")]
    async fn get_param_work_branch(&self) -> String {
        self.get_param_work_branch_impl().await
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
        crate::mcp::common::generate_api_docs_from_router(&tools, "Planner")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tools_match_common_list() {
        let tools = PlannerMcp::tool_router().list_all();
        let mut names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();
        names.sort();
        let mut expected = crate::mcp::planner_tools::ALL_TOOLS.to_vec();
        expected.sort();
        assert_eq!(
            names, expected,
            "planner tool router diverged from common list"
        );
    }
}
