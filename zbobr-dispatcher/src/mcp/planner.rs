use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::{
    ZbobrDispatcher,
    mcp::{
        common::DescriptionParam,
        traits::{CommonMcpImpl, PlannerMcpImpl},
    },
    task::RoleSession,
};

#[derive(Clone)]
pub struct PlannerMcp {
    session: RoleSession,
    tool_router: ToolRouter<Self>,
}

impl CommonMcpImpl for PlannerMcp {
    fn session(&self) -> &RoleSession {
        &self.session
    }

    fn role(&self) -> crate::task::Role {
        crate::task::Role::Planner
    }
}

impl PlannerMcpImpl for PlannerMcp {}

#[tool_router]
impl PlannerMcp {
    pub fn new(zbobr: ZbobrDispatcher, task_id: u64) -> Self {
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

    #[tool(description = "Get the current implementation plan for this task")]
    async fn get_plan(&self) -> String {
        self.get_plan_impl().await
    }

    #[tool(description = "Post or replace the implementation plan for this task")]
    async fn post_plan(&self, Parameters(params): Parameters<DescriptionParam>) -> String {
        self.post_plan_impl(&params.description).await
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
