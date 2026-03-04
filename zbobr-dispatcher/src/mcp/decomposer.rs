use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::{
    ZbobrDispatcherDyn,
    mcp::{
        common::MessageParam,
        traits::{DecomposerMcpImpl, CommonMcpImpl},
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

impl DecomposerMcpImpl for DecomposerMcp {}

#[tool_router]
impl DecomposerMcp {
    pub fn new(zbobr: ZbobrDispatcherDyn, task_id: u64) -> Self {
        Self {
            session: zbobr.role_session(task_id),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Report successful decomposition completion")]
    async fn report_done(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_done_impl(&params.message).await
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
}

#[tool_handler]
impl ServerHandler for DecomposerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Decomposer tools: create subtasks based on the decomposition plan.".to_string(),
            ),
            ..Default::default()
        }
    }
}

impl DecomposerMcp {
    /// Generate API documentation for decomposer tools
    pub fn generate_api_docs() -> String {
        let tools = Self::tool_router();
        crate::mcp::common::generate_api_docs_from_router(&tools, "Decomposer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tools_match_common_list() {
        let tools = DecomposerMcp::tool_router().list_all();
        let mut names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();
        names.sort();
        let mut expected = crate::mcp::decomposer_tools::ALL_TOOLS.to_vec();
        expected.sort();
        assert_eq!(
            names, expected,
            "decomposer tool router diverged from common list"
        );
    }
}
