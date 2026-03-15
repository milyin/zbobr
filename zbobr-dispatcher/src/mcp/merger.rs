use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::{
    ZbobrDispatcher,
    backend::TaskBackend,
    mcp::{
        common::{GetHistoryParam, MessageParam},
        traits::{CommonMcpImpl, MergerMcpImpl},
    },
    task::{Model, RoleSession, Tool},
};

#[derive(Clone)]
pub struct MergerMcp<TB: TaskBackend + Clone + Send + Sync + 'static> {
    session: RoleSession<TB>,
    tool_router: ToolRouter<Self>,
    tool: Tool,
    model: Model,
}

impl<TB: TaskBackend + Clone + Send + Sync + 'static> CommonMcpImpl for MergerMcp<TB> {
    type TB = TB;

    fn session(&self) -> &RoleSession<TB> {
        &self.session
    }

    fn role(&self) -> crate::task::Role {
        crate::task::Role::Merger
    }

    fn mcp_tool(&self) -> Tool {
        self.tool
    }

    fn mcp_model(&self) -> Model {
        self.model.clone()
    }
}

impl<TB: TaskBackend + Clone + Send + Sync + 'static> MergerMcpImpl for MergerMcp<TB> {}

#[tool_router]
impl<TB: TaskBackend + Clone + Send + Sync + 'static> MergerMcp<TB> {
    pub fn new(zbobr: ZbobrDispatcher, task_backend: TB, task_id: u64, tool: Tool, model: Model) -> Self {
        Self {
            session: zbobr.role_session(task_backend, task_id),
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

    #[tool(
        description = "Provide a brief and concise report of your results and finish your work. These reports add up to discussion and shorten the context for further agent calls, so they MUST be compact."
    )]
    async fn report_results(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_results_impl(&params.message).await
    }
}

#[tool_handler]
impl<TB: TaskBackend + Clone + Send + Sync + 'static> ServerHandler for MergerMcp<TB> {
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

impl<TB: TaskBackend + Clone + Send + Sync + 'static> MergerMcp<TB> {
    /// Generate API documentation for merger tools
    pub fn generate_api_docs() -> String {
        let tools = Self::tool_router();
        crate::mcp::common::generate_api_docs_from_router(&tools, "Merger")
    }
}
