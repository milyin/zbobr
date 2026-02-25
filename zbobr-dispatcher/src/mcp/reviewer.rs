use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::{
    Zbobr,
    mcp::{
        common::{
            CheckChecklistItemParam, DeleteChecklistItemParam, InsertChecklistItemParam,
            UpdateChecklistItemParam,
        },
        traits::{CommonMcpImpl, ReviewerMcpImpl},
    },
    task::RoleSession,
};

#[derive(Clone)]
pub struct ReviewerMcp {
    session: RoleSession,
    tool_router: ToolRouter<Self>,
}

impl CommonMcpImpl for ReviewerMcp {
    fn session(&self) -> &RoleSession {
        &self.session
    }

    fn role(&self) -> crate::task::Role {
        crate::task::Role::Reviewer
    }
}

impl ReviewerMcpImpl for ReviewerMcp {}

#[tool_router]
impl ReviewerMcp {
    pub fn new(zbobr: Zbobr, task_id: u64) -> Self {
        Self {
            session: zbobr.role_session(task_id),
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
    async fn report_error(
        &self,
        Parameters(params): Parameters<crate::mcp::common::MessageParam>,
    ) -> String {
        self.report_error_impl(&params.message).await
    }

    #[tool(description = "Get the current implementation plan for this task (read-only)")]
    async fn get_plan(&self) -> String {
        self.get_plan_impl().await
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
    async fn report_results(
        &self,
        Parameters(params): Parameters<crate::mcp::common::MessageParam>,
    ) -> String {
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
        crate::mcp::common::generate_api_docs_from_router(&tools, "Reviewer")
    }
}
