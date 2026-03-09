use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::{
    ZbobrDispatcherDyn,
    mcp::{
        common::GetHistoryParam,
        traits::{CommonMcpImpl, ReviewerMcpImpl},
    },
    task::{RoleSession, Model, Tool},
};

#[derive(Clone)]
pub struct ReviewerMcp {
    session: RoleSession,
    tool_router: ToolRouter<Self>,
    tool: Tool,
    model: Model,
}

impl CommonMcpImpl for ReviewerMcp {
    fn session(&self) -> &RoleSession {
        &self.session
    }

    fn role(&self) -> crate::task::Role {
        crate::task::Role::Reviewer
    }

    fn mcp_tool(&self) -> Tool {
        self.tool
    }

    fn mcp_model(&self) -> Model {
        self.model.clone()
    }
}

impl ReviewerMcpImpl for ReviewerMcp {}

#[tool_router]
impl ReviewerMcp {
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
        description = "Accept the review: the implementation is correct and the task is done. Provide a concise summary of what was reviewed and confirmed."
    )]
    async fn review_accept(
        &self,
        Parameters(params): Parameters<crate::mcp::common::MessageParam>,
    ) -> String {
        self.review_accept_impl(&params.message).await
    }

    #[tool(
        description = "Reject the review: the implementation has issues that need to be addressed. Provide a concise description of the problems found."
    )]
    async fn review_reject(
        &self,
        Parameters(params): Parameters<crate::mcp::common::MessageParam>,
    ) -> String {
        self.review_reject_impl(&params.message).await
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
