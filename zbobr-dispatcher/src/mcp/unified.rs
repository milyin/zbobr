use std::collections::HashSet;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::ToolCallContext, wrapper::Parameters},
    model::{
        CallToolRequestParams, CallToolResult, Content, ServerCapabilities, ServerInfo,
        Tool as McpToolDef,
    },
    tool, tool_router,
};
use rmcp::service::RequestContext;

use crate::{
    mcp::common::{
        AddChecklistItemParam, CheckChecklistItemParam, ConfigureWorktreeParam,
        DeleteChecklistItemParam, GetHistoryRecordParam, MessageParam,
    },
    mcp::traits::CommonMcpImpl,
    task::{Model, RoleSession, Tool},
};

/// A single unified MCP server that defines ALL possible tools and filters them
/// at runtime based on the role's `allowed_tools` set.
#[derive(Clone)]
pub struct UnifiedMcp {
    session: RoleSession,
    tool_router: ToolRouter<Self>,
    allowed_tools: HashSet<String>,
    role_name: String,
    tool: Tool,
    model: Model,
    stage_name: String,
}

impl CommonMcpImpl for UnifiedMcp {
    fn session(&self) -> &RoleSession {
        &self.session
    }

    fn role_name(&self) -> &str {
        &self.role_name
    }

    fn mcp_tool(&self) -> Tool {
        self.tool
    }

    fn mcp_model(&self) -> Model {
        self.model.clone()
    }

    fn stage_name(&self) -> &str {
        &self.stage_name
    }
}

/// All possible tool names across all roles.
pub const ALL_TOOL_NAMES: &[&str] = &[
    "get_history_index",
    "get_history_record",
    "report_success",
    "report_failure",
    "stop_with_error",
    "stop_with_question",
    "configure_worktree",
    "get_checklist",
    "add_checklist_item",
    "check_checklist_item",
    "delete_checklist_item",
];

#[tool_router]
impl UnifiedMcp {
    pub fn new(
        session: RoleSession,
        allowed_tools: HashSet<String>,
        role_name: String,
        tool: Tool,
        model: Model,
        stage_name: String,
    ) -> Self {
        Self {
            session,
            tool_router: Self::tool_router(),
            allowed_tools,
            role_name,
            tool,
            model,
            stage_name,
        }
    }

    // -- All tools defined here. Filtering happens in ServerHandler impl. --

    #[tool(
        description = "Get the full history index: position, author (stage or 'user'), record type (task/success/failure/question/error/other), hidden flag, and summary for each record."
    )]
    async fn get_history_index(&self) -> String {
        self.get_history_index_impl().await
    }

    #[tool(description = "Get a single history record by position index. Position 0 is the task description.")]
    async fn get_history_record(
        &self,
        Parameters(params): Parameters<GetHistoryRecordParam>,
    ) -> String {
        self.get_history_record_impl(params.index).await
    }

    #[tool(
        description = "Provide a brief and concise report of your results and finish your work. These reports add up to discussion and shorten the context for further agent calls, so they MUST be compact."
    )]
    async fn report_success(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_success_impl(&params.message).await
    }

    #[tool(
        description = "Report a failure or rejection, returning the task for re-work or re-planning. Provide a concise description of the problems found."
    )]
    async fn report_failure(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.report_failure_impl(&params.message).await
    }

    #[tool(description = "Report an error to the user and pause task processing")]
    async fn stop_with_error(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.stop_with_error_impl(&params.message).await
    }

    #[tool(
        description = "Post a question to the user and pause task processing until user responds"
    )]
    async fn stop_with_question(&self, Parameters(params): Parameters<MessageParam>) -> String {
        self.stop_with_question_impl(&params.message).await
    }

    #[tool(
        description = "Configure worktree parameters: destination repository, destination branch, and/or work branch postfix. All three are optional; only provided values are updated."
    )]
    async fn configure_worktree(
        &self,
        Parameters(params): Parameters<ConfigureWorktreeParam>,
    ) -> String {
        self.configure_worktree_impl(
            params.destination_repository,
            params.destination_branch,
            params.work_branch_postfix,
        )
        .await
    }

    #[tool(description = "Get the task checklist (unchecked items only)")]
    async fn get_checklist(&self) -> String {
        self.get_checklist_impl().await
    }

    #[tool(description = "Add a new checklist item (always appended, always unchecked)")]
    async fn add_checklist_item(
        &self,
        Parameters(params): Parameters<AddChecklistItemParam>,
    ) -> String {
        self.add_checklist_item_impl(&params.id, &params.text).await
    }

    #[tool(description = "Mark a checklist item as checked")]
    async fn check_checklist_item(
        &self,
        Parameters(params): Parameters<CheckChecklistItemParam>,
    ) -> String {
        self.check_checklist_item_impl(&params.id).await
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
}

// Manual ServerHandler implementation with tool filtering
impl ServerHandler for UnifiedMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(format!(
                "{} tools: MCP server for task management.",
                self.role_name
            )),
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, McpError> {
        let all_tools = self.tool_router.list_all();
        let filtered: Vec<McpToolDef> = all_tools
            .into_iter()
            .filter(|t| self.allowed_tools.contains(t.name.as_ref()))
            .collect();
        Ok(rmcp::model::ListToolsResult {
            meta: None,
            tools: filtered,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tool_name = request.name.as_ref();
        if !self.allowed_tools.contains(tool_name) {
            return Ok(CallToolResult {
                content: vec![Content::text(format!(
                    "Error: tool '{}' is not available for role '{}'",
                    tool_name, self.role_name
                ))],
                structured_content: None,
                is_error: Some(true),
                meta: None,
            });
        }
        let tcc = ToolCallContext::new(self, request, context);
        self.tool_router.call(tcc).await.map_err(McpError::from)
    }
}

impl UnifiedMcp {
    /// Generate API documentation for the tools available to this role.
    pub fn generate_api_docs(allowed_tools: &HashSet<String>) -> String {
        let router = Self::tool_router();
        let all_tools = router.list_all();
        let filtered: Vec<_> = all_tools
            .iter()
            .filter(|t| allowed_tools.contains(t.name.as_ref()))
            .collect();

        let mut doc = String::from("## MCP API\n\nAvailable tools (all pre-scoped to your task):\n\n");
        for tool in filtered {
            doc.push_str(&format!("### `{}`\n\n", tool.name));
            doc.push_str(&format!(
                "{}\n\n",
                tool.description.as_deref().unwrap_or("No description")
            ));
            doc.push_str("---\n\n");
        }
        doc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tool_names_match_router() {
        let router = UnifiedMcp::tool_router();
        let mut router_names: Vec<_> = router.list_all().iter().map(|t| t.name.to_string()).collect();
        router_names.sort();
        let mut expected: Vec<_> = ALL_TOOL_NAMES.iter().map(|s| s.to_string()).collect();
        expected.sort();
        assert_eq!(router_names, expected, "ALL_TOOL_NAMES diverged from router");
    }

    #[test]
    fn filtering_works() {
        let router = UnifiedMcp::tool_router();
        let allowed: HashSet<String> = ["get_history_index", "stop_with_error"].iter().map(|s| s.to_string()).collect();
        let all_tools = router.list_all();
        let filtered: Vec<_> = all_tools
            .iter()
            .filter(|t| allowed.contains(t.name.as_ref()))
            .collect();
        assert_eq!(filtered.len(), 2);
    }
}
