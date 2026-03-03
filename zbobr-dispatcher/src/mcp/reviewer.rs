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
    pub fn new(zbobr: ZbobrDispatcherDyn, task_id: u64) -> Self {
        Self {
            session: zbobr.role_session(task_id),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get the plan and following comments. Optional offset: 0 = latest plan (default), -1 = previous plan, etc.")]
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
