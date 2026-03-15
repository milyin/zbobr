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
        common::GetHistoryParam,
        traits::{CommonMcpImpl, TesterMcpImpl},
    },
    task::{Model, RoleSession, Tool},
};

#[derive(Clone)]
pub struct TesterMcp<TB: TaskBackend + Clone + Send + Sync + 'static> {
    session: RoleSession<TB>,
    tool_router: ToolRouter<Self>,
    tool: Tool,
    model: Model,
}

impl<TB: TaskBackend + Clone + Send + Sync + 'static> CommonMcpImpl for TesterMcp<TB> {
    type TB = TB;

    fn session(&self) -> &RoleSession<TB> {
        &self.session
    }

    fn role(&self) -> crate::task::Role {
        crate::task::Role::Tester
    }

    fn mcp_tool(&self) -> Tool {
        self.tool
    }

    fn mcp_model(&self) -> Model {
        self.model.clone()
    }
}

impl<TB: TaskBackend + Clone + Send + Sync + 'static> TesterMcpImpl for TesterMcp<TB> {}

#[tool_router]
impl<TB: TaskBackend + Clone + Send + Sync + 'static> TesterMcp<TB> {
    pub fn new(
        zbobr: ZbobrDispatcher,
        task_backend: TB,
        task_id: u64,
        tool: Tool,
        model: Model,
    ) -> Self {
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
    async fn report_error(
        &self,
        Parameters(params): Parameters<crate::mcp::common::MessageParam>,
    ) -> String {
        self.report_error_impl(&params.message).await
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
        description = "Accept the testing: all tests pass and requirements are met. Provide a concise summary of all testing performed and results."
    )]
    async fn test_accept(
        &self,
        Parameters(params): Parameters<crate::mcp::common::MessageParam>,
    ) -> String {
        self.test_accept_impl(&params.message).await
    }

    #[tool(
        description = "Reject the testing: tests failed or requirements not met. Provide a concise description of all test failures and what needs to be fixed."
    )]
    async fn test_reject(
        &self,
        Parameters(params): Parameters<crate::mcp::common::MessageParam>,
    ) -> String {
        self.test_reject_impl(&params.message).await
    }
}

#[tool_handler]
impl<TB: TaskBackend + Clone + Send + Sync + 'static> ServerHandler for TesterMcp<TB> {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Tester tools: run comprehensive tests, add testing remarks to checklist."
                    .to_string(),
            ),
            ..Default::default()
        }
    }
}

impl<TB: TaskBackend + Clone + Send + Sync + 'static> TesterMcp<TB> {
    /// Generate API documentation for tester tools
    pub fn generate_api_docs() -> String {
        let tools = Self::tool_router();
        crate::mcp::common::generate_api_docs_from_router(&tools, "Tester")
    }
}
