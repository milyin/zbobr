use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use std::sync::Arc;

use crate::{
    ZbobrDispatcher,
    mcp::{
        common::{
            GetHistoryParam, MessageParam, SetDestinationBranchParam, SetDestinationRepositoryParam,
        },
        traits::{CommonMcpImpl, PreparatorMcpImpl},
    },
    task::{RoleSession, Model, Tool},
};

#[derive(Clone)]
pub struct PreparatorMcp {
    session: RoleSession,
    tool_router: ToolRouter<Self>,
    tool: Tool,
    model: Model,
}

impl CommonMcpImpl for PreparatorMcp {
    fn session(&self) -> &RoleSession {
        &self.session
    }

    fn role(&self) -> crate::task::Role {
        crate::task::Role::Preparator
    }

    fn mcp_tool(&self) -> Tool {
        self.tool
    }

    fn mcp_model(&self) -> Model {
        self.model.clone()
    }
}

impl PreparatorMcpImpl for PreparatorMcp {}

#[tool_router]
impl PreparatorMcp {
    pub fn new(zbobr: ZbobrDispatcher, task_backend: Arc<dyn crate::backend::TaskBackend>, task_id: u64, tool: Tool, model: Model) -> Self {
        Self {
            session: zbobr.role_session(task_backend, task_id),
            tool_router: Self::tool_router(),
            tool,
            model,
        }
    }

    #[tool(
        description = "Get task history chunk. Optional offset: chunk index (0 = oldest, omitted = latest). Response includes current_chunk and last_chunk for navigation. Returns task description if no history exists yet."
    )]
    async fn get_history(&self, Parameters(params): Parameters<GetHistoryParam>) -> String {
        self.get_history_impl(params.offset).await
    }

    #[tool(description = "Report an error to the user and pause task processing")]
    async fn report_error(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_error_impl(&params.message).await
    }

    #[tool(description = "Get the destination repository URL for this task (read-only)")]
    async fn get_param_destination_repository(&self) -> String {
        self.get_param_destination_repository_impl().await
    }

    #[tool(
        description = "Set the destination repository for this task (full git URL, local path, or 'owner/repo')"
    )]
    async fn set_param_destination_repository(
        &self,
        Parameters(params): Parameters<SetDestinationRepositoryParam>,
    ) -> String {
        self.set_param_destination_repository_impl(params.value)
            .await
    }

    #[tool(description = "Get the destination branch name for this task (read-only)")]
    async fn get_param_destination_branch(&self) -> String {
        self.get_param_destination_branch_impl().await
    }

    #[tool(description = "Set the destination branch name for this task (e.g. 'main')")]
    async fn set_param_destination_branch(
        &self,
        Parameters(params): Parameters<SetDestinationBranchParam>,
    ) -> String {
        self.set_param_destination_branch_impl(params.value).await
    }

    #[tool(
        description = "Set the work branch postfix for this task (the postfix segment, e.g. 'implement-feature')"
    )]
    async fn set_param_work_branch_postfix(
        &self,
        Parameters(params): Parameters<SetDestinationBranchParam>,
    ) -> String {
        self.set_param_work_branch_postfix_impl(params.value).await
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
impl ServerHandler for PreparatorMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Preparator tools: read task description and set implementation parameters."
                    .to_string(),
            ),
            ..Default::default()
        }
    }
}

impl PreparatorMcp {
    /// Generate API documentation for preparator tools
    pub fn generate_api_docs() -> String {
        let tools = Self::tool_router();
        crate::mcp::common::generate_api_docs_from_router(&tools, "Preparator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tools_match_common_list() {
        let tools = PreparatorMcp::tool_router().list_all();
        let mut names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();
        names.sort();
        let mut expected = crate::mcp::preparator_tools::ALL_TOOLS.to_vec();
        expected.sort();
        assert_eq!(
            names, expected,
            "preparator tool router diverged from common list"
        );
    }
}
