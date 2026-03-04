use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::{
    ZbobrDispatcherDyn,
    mcp::{
        common::GetPlanParam,
        traits::{CommonMcpImpl, TesterMcpImpl},
    },
    task::RoleSession,
};

#[derive(Clone)]
pub struct TesterMcp {
    session: RoleSession,
    tool_router: ToolRouter<Self>,
}

impl CommonMcpImpl for TesterMcp {
    fn session(&self) -> &RoleSession {
        &self.session
    }

    fn role(&self) -> crate::task::Role {
        crate::task::Role::Tester
    }
}

impl TesterMcpImpl for TesterMcp {}

#[tool_router]
impl TesterMcp {
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
impl ServerHandler for TesterMcp {
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

impl TesterMcp {
    /// Generate API documentation for tester tools
    pub fn generate_api_docs() -> String {
        let tools = Self::tool_router();
        crate::mcp::common::generate_api_docs_from_router(&tools, "Tester")
    }
}
