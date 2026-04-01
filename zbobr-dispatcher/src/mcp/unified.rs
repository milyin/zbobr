use std::collections::HashSet;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::ToolCallContext, wrapper::Parameters},
    model::{
        CallToolRequestParams, CallToolResult, Content, ServerCapabilities, ServerInfo,
        Tool as McpToolDef,
    },
    service::RequestContext,
    tool, tool_router,
};
use zbobr_api::config_tools::McpTool;

use crate::{
    mcp::{
        common::{
            AddChecklistItemParam, CheckChecklistItemParam, DeleteCtxRecParam, GetCtxRecParam,
            MessageParam, ReportParam,
        },
        traits::CommonMcpImpl,
    },
    task::{Model, RoleSession, Tool},
};

/// A single unified MCP server that defines ALL possible tools and filters them
/// at runtime based on the role's `allowed_tools` set.
#[derive(Clone)]
pub struct UnifiedMcp {
    session: RoleSession,
    tool_router: ToolRouter<Self>,
    allowed_tools: HashSet<McpTool>,
    role_name: String,
    tool: Tool,
    model: Model,
    stage_name: String,
    /// Pipeline name for this session.
    pipeline_name: String,
    /// Pipeline run ID for this session.
    pipeline_run_id: u64,
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

    fn pipeline_name(&self) -> &str {
        &self.pipeline_name
    }

    fn pipeline_run_id(&self) -> u64 {
        self.pipeline_run_id
    }
}

/// All possible static tool names across all roles.
pub const ALL_TOOL_NAMES: &[&str] = zbobr_api::config_tools::ALL_TOOL_NAMES;

#[tool_router]
impl UnifiedMcp {
    pub fn new(
        session: RoleSession,
        allowed_tools: HashSet<McpTool>,
        role_name: String,
        tool: Tool,
        model: Model,
        stage_name: String,
        pipeline_name: String,
        pipeline_run_id: u64,
    ) -> Self {
        Self {
            session,
            tool_router: Self::tool_router(),
            allowed_tools,
            role_name,
            tool,
            model,
            stage_name,
            pipeline_name,
            pipeline_run_id,
        }
    }

    // -- All tools defined here. Filtering happens in ServerHandler impl. --

    #[tool(
        description = "Report successful completion of the given task. Use when all assigned work is fully done. The brief summary is stored as a context record; the full report is stored as a file."
    )]
    async fn report_success(&self, Parameters(params): Parameters<ReportParam>) -> String {
        self.report_success_impl(&params.brief, &params.full_report)
            .await
    }

    #[tool(
        description = "Report a failure of implementing the task. Use when work cannot be completed or results are unacceptable. The brief summary is stored as a context record; the full report is stored as a file."
    )]
    async fn report_failure(&self, Parameters(params): Parameters<ReportParam>) -> String {
        self.report_failure_impl(&params.brief, &params.full_report)
            .await
    }

    #[tool(
        description = "Report intermediate progress of the task. Use when some work is done and some remains. The brief summary is stored as a context record; the full report is stored as a file."
    )]
    async fn report_intermediate(&self, Parameters(params): Parameters<ReportParam>) -> String {
        self.report_intermediate_impl(&params.brief, &params.full_report)
            .await
    }

    #[tool(
        description = "Add a new unchecked checklist item to the current stage context. Checklist items are considered as elaboration of the report provided. Brief summary is stored as context record text; full report is stored as a file."
    )]
    async fn add_checklist_item(
        &self,
        Parameters(params): Parameters<AddChecklistItemParam>,
    ) -> String {
        self.add_checklist_item_impl(&params.brief, &params.full_report)
            .await
    }

    #[tool(
        description = "Mark a checklist item as checked (done). Accepts either a numeric id or a string like 'ctx_rec_5'."
    )]
    async fn check_checklist_item(
        &self,
        Parameters(params): Parameters<CheckChecklistItemParam>,
    ) -> String {
        self.check_checklist_item_impl(&params.id).await
    }

    #[tool(
        description = "Delete a context record by its id. Accepts either a numeric id or a string like 'ctx_rec_5'."
    )]
    async fn delete_ctx_rec(&self, Parameters(params): Parameters<DeleteCtxRecParam>) -> String {
        self.delete_ctx_rec_impl(&params.id).await
    }

    #[tool(
        description = "Get the content of a context record by its id. Returns the full report content if available, or the brief summary. Accepts either a numeric id or a string like 'ctx_rec_5'."
    )]
    async fn get_ctx_rec(&self, Parameters(params): Parameters<GetCtxRecParam>) -> String {
        self.get_ctx_rec_impl(&params.id).await
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
            .filter(|t| {
                t.name
                    .as_ref()
                    .parse::<McpTool>()
                    .map(|tool| self.allowed_tools.contains(&tool))
                    .unwrap_or(false)
            })
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
        let requested_tool = tool_name.parse::<McpTool>().ok();
        if requested_tool.is_none_or(|tool| !self.allowed_tools.contains(&tool)) {
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
    pub fn generate_api_docs(allowed_tools: &HashSet<McpTool>) -> String {
        let router = Self::tool_router();
        let all_tools = router.list_all();
        let filtered: Vec<_> = all_tools
            .iter()
            .filter(|t| {
                t.name
                    .as_ref()
                    .parse::<McpTool>()
                    .map(|tool| allowed_tools.contains(&tool))
                    .unwrap_or(false)
            })
            .collect();

        let mut doc =
            String::from("## MCP API\n\nAvailable tools (all pre-scoped to your task):\n\n");
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
        let mut router_names: Vec<_> = router
            .list_all()
            .iter()
            .map(|t| t.name.to_string())
            .collect();
        router_names.sort();
        let mut expected: Vec<_> = ALL_TOOL_NAMES.iter().map(|s| s.to_string()).collect();
        expected.sort();
        assert_eq!(
            router_names, expected,
            "ALL_TOOL_NAMES diverged from router"
        );
    }

    #[test]
    fn filtering_works() {
        let router = UnifiedMcp::tool_router();
        let allowed: HashSet<McpTool> = [McpTool::AddChecklistItem, McpTool::StopWithError]
            .into_iter()
            .collect();
        let all_tools = router.list_all();
        let filtered: Vec<_> = all_tools
            .iter()
            .filter(|t| {
                t.name
                    .as_ref()
                    .parse::<McpTool>()
                    .map(|tool| allowed.contains(&tool))
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(filtered.len(), 2);
    }
}
