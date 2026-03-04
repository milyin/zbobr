use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::{
    ZbobrDispatcherDyn,
    mcp::{
        common::{GetPlanParam, MessageParam},
        traits::{DecomposePlannerMcpImpl, CommonMcpImpl},
    },
    task::RoleSession,
};

#[derive(Clone)]
pub struct DecomposePlannerMcp {
    session: RoleSession,
    tool_router: ToolRouter<Self>,
}

impl CommonMcpImpl for DecomposePlannerMcp {
    fn session(&self) -> &RoleSession {
        &self.session
    }

    fn role(&self) -> crate::task::Role {
        crate::task::Role::DecomposePlanner
    }
}

impl DecomposePlannerMcpImpl for DecomposePlannerMcp {}

#[tool_router]
impl DecomposePlannerMcp {
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

    #[tool(description = "Post the decomposition plan for this task")]
    async fn post_plan(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.post_plan_impl(&params.message).await
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
impl ServerHandler for DecomposePlannerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "DecomposePlanner tools: create a plan for decomposing umbrella tasks into subtasks.".to_string(),
            ),
            ..Default::default()
        }
    }
}

impl DecomposePlannerMcp {
    /// Generate API documentation for decompose_planner tools
    pub fn generate_api_docs() -> String {
        let tools = Self::tool_router();
        crate::mcp::common::generate_api_docs_from_router(&tools, "DecomposePlanner")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tools_match_common_list() {
        let tools = DecomposePlannerMcp::tool_router().list_all();
        let mut names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();
        names.sort();
        let mut expected = crate::mcp::decompose_planner_tools::ALL_TOOLS.to_vec();
        expected.sort();
        assert_eq!(
            names, expected,
            "decompose_planner tool router diverged from common list"
        );
    }
}
