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
            DeleteChecklistItemParam, DescriptionParam, GetPlanParam, InsertChecklistItemParam,
            MessageParam, UpdateChecklistItemParam,
        },
        traits::{CommonMcpImpl, PlannerMcpImpl},
    },
    task::RoleSession,
};

#[derive(Clone)]
pub struct DecomposerMcp {
    session: RoleSession,
    tool_router: ToolRouter<Self>,
}

impl CommonMcpImpl for DecomposerMcp {
    fn session(&self) -> &RoleSession {
        &self.session
    }

    fn role(&self) -> crate::task::Role {
        crate::task::Role::Decomposer
    }
}

impl PlannerMcpImpl for DecomposerMcp {}

#[tool_router]
impl DecomposerMcp {
    pub fn new(zbobr: ZbobrDispatcherDyn, task_id: u64) -> Self {
        Self {
            session: zbobr.role_session(task_id),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Get the plan and following comments (analysis comments excluded). Optional offset: 0 = latest plan (default), -1 = previous plan, etc."
    )]
    async fn get_plan(&self, Parameters(params): Parameters<GetPlanParam>) -> String {
        self.get_plan_impl(params.offset.unwrap_or(0)).await
    }

    #[tool(description = "Get all analysis comments for this task in chronological order")]
    async fn get_analysis(&self) -> String {
        self.get_analysis_impl().await
    }

    #[tool(description = "Post the decomposition plan for this task and finish your session")]
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

    #[tool(description = "Post a message to the user and pause task processing until user responds")]
    async fn ask_user(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.ask_user_impl(&params.message).await
    }

    #[tool(description = "Provide a brief and concise report of your work and finish the session")]
    async fn report_results(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_results_impl(&params.message).await
    }
}

#[tool_handler]
impl DecomposerMcp {
    /// Generate API documentation for decomposer tools
    pub fn generate_api_docs() -> String {
        let tools = Self::tool_router();
        crate::mcp::common::generate_api_docs_from_router(&tools, "Decomposer")
    }
}

impl ServerHandler for DecomposerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Decomposer tools: analyze umbrella task and create subtasks for execution.".to_string(),
            ),
            ..Default::default()
        }
    }
}
